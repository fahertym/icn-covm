use crate::governance::ProposalLifecycle;
use crate::governance::comments;
use crate::governance::proposal::Proposal;
use crate::storage::auth::AuthContext;
use crate::storage::traits::{Storage, StorageExtensions, StorageBackend, AsyncStorageExtensions, JsonStorage};
use crate::storage::errors::StorageError;
use crate::vm::VM;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{Filter, Rejection, Reply};

/// Represents a proposal with all of its metadata for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ProposalResponse {
    pub id: String,
    pub title: String,
    pub creator: String,
    pub status: String,
    pub created_at: String,
    pub votes: VoteCounts,
    pub quorum_percentage: f64,
    pub threshold_percentage: f64,
    pub execution_result: Option<String>,
}

/// Vote count information
#[derive(Debug, Serialize, Deserialize)]
pub struct VoteCounts {
    pub vote_count: u32,
    pub breakdown: VoteBreakdown,
}

/// Vote breakdown information
#[derive(Debug, Serialize, Deserialize)]
pub struct VoteBreakdown {
    pub yes: u32,
    pub no: u32,
    pub abstain: u32,
    pub total: u32,
}

/// Comment metadata for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct CommentResponse {
    pub id: String,
    pub author: String,
    pub timestamp: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub tags: Vec<String>,
    pub reactions: HashMap<String, u32>,
    pub hidden: bool,
    pub edit_count: usize,
}

/// Comment version history for API
#[derive(Debug, Serialize, Deserialize)]
pub struct CommentVersionResponse {
    content: String,
    timestamp: String,
}

/// Proposal summary for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ProposalSummary {
    id: String,
    title: String,
    status: String,
    comment_count: usize,
    vote_count: u32,
    vote_details: VoteCounts,
    top_participants: Vec<Participant>,
    last_activity: String,
}

/// Participant information for summaries
#[derive(Debug, Serialize, Deserialize)]
pub struct Participant {
    id: String,
    comment_count: u32,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
}

/// Query parameters for filtering hidden comments
#[derive(Debug, Serialize, Deserialize)]
pub struct ShowHiddenQuery {
    show_hidden: Option<bool>,
}

/// Also import VoteType for vote counting
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VoteType {
    Yes,
    No,
    Abstain,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Vote {
    pub voter: String,
    pub vote: VoteType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Returns all the proposal API routes
pub fn get_routes<S>(vm: Arc<VM<Arc<Mutex<S>>>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    // Create routes for API endpoints
    let proposals_route = warp::path!("proposals" / String)
        .and(with_vm(vm.clone()))
        .and_then(get_proposal);

    let comments_route = warp::path!("proposals" / String / "comments")
        .and(with_vm(vm.clone()))
        .and(warp::query::<ShowHiddenQuery>())
        .and_then(get_proposal_comments);

    let summary_route = warp::path!("proposals" / String / "summary")
        .and(with_vm(vm.clone()))
        .and_then(get_proposal_summary);

    // Combine all proposal routes
    proposals_route
        .or(comments_route)
        .or(summary_route)
}

/// Dependency injection helper for the VM
fn with_vm<S>(
    vm: Arc<VM<Arc<Mutex<S>>>>,
) -> impl Filter<Extract = (Arc<VM<Arc<Mutex<S>>>>,), Error = Infallible> + Clone
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    warp::any().map(move || vm.clone())
}

/// Handler for GET /proposals/{id}
async fn get_proposal<S>(id: String, vm: Arc<VM<Arc<Mutex<S>>>>) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_mutex = vm.storage_backend.clone();
    let storage = storage_mutex.expect("Storage backend not available").lock().await;

    // Load proposal
    let proposal_result = load_proposal_from_governance(&vm, &id).await;

    match proposal_result {
        Ok(proposal) => {
            // Count votes
            let (yes_votes, no_votes, abstain_votes) = count_votes(&vm, &id).await.unwrap_or((0, 0, 0));
            let total_votes = yes_votes + no_votes + abstain_votes;

            // Calculate percentages
            let quorum_percentage = 0.0; // Would need real data from lifecycle
            let threshold_percentage = if total_votes > 0 {
                (yes_votes as f64 / total_votes as f64) * 100.0
            } else {
                0.0
            };

            // Build response
            let vote_count = total_votes as u32;
            
            let breakdown = VoteBreakdown {
                yes: yes_votes as u32,
                no: no_votes as u32,
                abstain: abstain_votes as u32,
                total: total_votes as u32,
            };

            let response = ProposalResponse {
                id: proposal.id,
                title: "".to_string(), // Would need to fetch from lifecycle
                creator: proposal.creator,
                status: format!("{:?}", proposal.status),
                created_at: proposal.created_at.to_rfc3339(),
                votes: VoteCounts {
                    vote_count,
                    breakdown,
                },
                quorum_percentage,
                threshold_percentage,
                execution_result: proposal.execution_result,
            };

            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let error = ErrorResponse {
                message: format!("Failed to load proposal: {}", e),
            };
            Ok(warp::reply::json(&error))
        }
    }
}

/// Handler for GET /proposals/{id}/comments
async fn get_proposal_comments<S>(
    id: String,
    vm: Arc<VM<Arc<Mutex<S>>>>,
    query: ShowHiddenQuery,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_mutex = vm.storage_backend.clone();
    let storage = storage_mutex.expect("Storage backend not available").lock().await;

    // Create a null auth context for read-only operations
    let auth_context = None;
    let show_hidden = query.show_hidden.unwrap_or(false);

    // Pass the show_hidden parameter to control visibility of hidden comments
    match comments::fetch_comments_threaded(
        &vm,
        &id,
        auth_context,
        show_hidden,
    ) {
        Ok(comments) => {
            // Convert to API response format
            let comment_responses: Vec<CommentResponse> = comments
                .values()
                .map(|comment| CommentResponse {
                    id: comment.id.clone(),
                    author: comment.author.clone(),
                    timestamp: comment.timestamp.to_rfc3339(),
                    content: comment.content.clone(),
                    reply_to: comment.reply_to.clone(),
                    tags: comment.tags.clone(),
                    reactions: comment.reactions.clone(),
                    hidden: comment.hidden,
                    edit_count: comment.edit_history.len() - 1, // First version is not an edit
                })
                .collect();
            
            Ok(warp::reply::json(&comment_responses))
        }
        Err(e) => {
            let error = ErrorResponse {
                message: format!("Failed to load comments: {}", e),
            };
            Ok(warp::reply::json(&error))
        }
    }
}

/// Handler for GET /proposals/{id}/summary
async fn get_proposal_summary<S>(id: String, vm: Arc<VM<Arc<Mutex<S>>>>) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_mutex = vm.storage_backend.clone();
    let storage = storage_mutex.expect("Storage backend not available").lock().await;

    // Load proposal and comments
    let proposal_result = load_proposal_from_governance(&vm, &id).await;
    let comments_result =
        crate::governance::comments::fetch_comments_threaded(&vm, &id, None, false);

    if let (Ok(proposal), Ok(comments)) = (&proposal_result, &comments_result) {
        // Count votes
        let (yes_votes, no_votes, abstain_votes) = count_votes(&vm, &id).await.unwrap_or((0, 0, 0));

        let total_votes = yes_votes + no_votes + abstain_votes;

        // Find most active participants
        let mut participant_activity: HashMap<String, u32> = HashMap::new();
        for comment in comments.values() {
            *participant_activity
                .entry(comment.author.clone())
                .or_insert(0) += 1;
        }

        // Convert to vector and sort
        let mut participants: Vec<(String, u32)> = participant_activity.into_iter().collect();
        participants.sort_by(|a, b| b.1.cmp(&a.1));

        // Build top participants list (max 5)
        let top_participants: Vec<Participant> = participants
            .into_iter()
            .take(5)
            .map(|(id, count)| Participant {
                id,
                comment_count: count,
            })
            .collect();

        // Get last activity timestamp
        let last_activity = comments
            .values()
            .map(|c| c.timestamp)
            .max()
            .unwrap_or(proposal.created_at)
            .to_rfc3339();

        // Build response
        let summary = ProposalSummary {
            id: proposal.id.clone(),
            title: "".to_string(), // Would need to fetch from lifecycle
            status: format!("{:?}", proposal.status),
            comment_count: comments.len(),
            vote_count: total_votes as u32,
            vote_details: VoteCounts {
                vote_count: total_votes as u32,
                breakdown: VoteBreakdown {
                    yes: yes_votes as u32,
                    no: no_votes as u32,
                    abstain: abstain_votes as u32,
                    total: total_votes as u32,
                },
            },
            top_participants,
            last_activity,
        };

        Ok(warp::reply::json(&summary))
    } else {
        // Make a clone of the results to avoid move errors
        let proposal_err = proposal_result.as_ref().err().map(|e| format!("{}", e));
        let comments_err = comments_result.as_ref().err().map(|e| format!("{}", e));

        // Handle errors
        let error_message = match (proposal_err, comments_err) {
            (Some(e), _) => format!("Failed to load proposal: {}", e),
            (_, Some(e)) => format!("Failed to load comments: {}", e),
            _ => "Unknown error".to_string(),
        };

        let error = ErrorResponse {
            message: error_message,
        };

        Ok(warp::reply::json(&error))
    }
}

/// Error handler for API rejections
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let error = ErrorResponse {
        message: format!("API error: {:?}", err),
    };

    Ok(warp::reply::json(&error))
}

/// Loads a proposal from storage
async fn load_proposal_from_governance<S>(vm: &VM<Arc<Mutex<S>>>, id: &str) -> Result<Proposal, String>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_mutex = vm.storage_backend.clone();
    let storage = storage_mutex.expect("Storage backend not available").lock().await;
    
    let proposal_key = format!("proposals/{}/metadata", id);
    
    match storage.get_json::<ProposalLifecycle>(None, "governance", &proposal_key) {
        Ok(lifecycle) => {
            // Convert the lifecycle to a proposal
            let proposal = Proposal {
                id: lifecycle.id.clone(),
                creator: lifecycle.creator.clone(),
                created_at: lifecycle.created_at,
                status: lifecycle.state.into(),
                execution_result: lifecycle.execution_status.map(|status| format!("{:?}", status)),
            };
            Ok(proposal)
        },
        Err(e) => Err(format!("Failed to retrieve or deserialize proposal: {}", e)),
    }
}

/// Count votes for a proposal
async fn count_votes<S>(
    vm: &Arc<VM<Arc<Mutex<S>>>>,
    proposal_id: &str,
) -> Result<(usize, usize, usize), Box<dyn std::error::Error>>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_mutex = vm.storage_backend.clone();
    let storage = storage_mutex.expect("Storage backend not available").lock().await;
    
    let votes_key = format!("proposals/{}/votes", proposal_id);
    let mut yes_votes = 0;
    let mut no_votes = 0;
    let mut abstain_votes = 0;
    
    // Get votes if available
    match storage.get_json::<HashMap<String, Vote>>(None, "governance", &votes_key) {
        Ok(votes) => {
            for vote in votes.values() {
                match vote.vote {
                    VoteType::Yes => yes_votes += 1,
                    VoteType::No => no_votes += 1,
                    VoteType::Abstain => abstain_votes += 1,
                }
            }
            Ok((yes_votes, no_votes, abstain_votes))
        },
        Err(StorageError::NotFound { .. }) => {
            // No votes yet
            Ok((0, 0, 0))
        },
        Err(e) => Err(Box::new(e)),
    }
}

/// Handler for GET /proposals
async fn list_proposals<S>(
    vm: Arc<VM<Arc<Mutex<S>>>>,
    query: ProposalQuery,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_mutex = vm.storage_backend.clone();
    let storage = storage_mutex.expect("Storage backend not available").lock().await;
    
    // ... existing code ...
}

/// Handler for POST /proposals
async fn create_proposal<S>(
    proposal: NewProposal,
    vm: Arc<VM<Arc<Mutex<S>>>>,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_mutex = vm.storage_backend.clone();
    let mut storage = storage_mutex.expect("Storage backend not available").lock().await;
    
    // ... existing code ...
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposalQuery {
    pub status: Option<String>,
    pub creator: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewProposal {
    pub title: String,
    pub description: String,
    pub proposed_code: Option<String>,
}
