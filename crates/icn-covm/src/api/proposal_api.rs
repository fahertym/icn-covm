use crate::cli::proposal::{count_votes, fetch_comments_threaded, load_proposal_from_governance};
use crate::governance::proposal::Proposal;
use crate::storage::auth::AuthContext;
use crate::storage::traits::{Storage, StorageExtensions};
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
struct ProposalResponse {
    id: String,
    title: String,
    creator: String,
    status: String,
    created_at: String,
    votes: VoteCounts,
    quorum_percentage: f64,
    threshold_percentage: f64,
    execution_result: Option<String>,
}

/// Vote count information
#[derive(Debug, Serialize, Deserialize)]
struct VoteCounts {
    yes: u32,
    no: u32,
    abstain: u32,
    total: u32,
}

/// Comment metadata for API responses
#[derive(Debug, Serialize, Deserialize)]
struct CommentResponse {
    id: String,
    author: String,
    timestamp: String,
    content: String,
    reply_to: Option<String>,
    tags: Vec<String>,
    reactions: HashMap<String, u32>,
    hidden: bool,
    edit_count: usize,
}

/// Comment version history for API
#[derive(Debug, Serialize, Deserialize)]
struct CommentVersionResponse {
    content: String,
    timestamp: String,
}

/// Proposal summary for API responses
#[derive(Debug, Serialize, Deserialize)]
struct ProposalSummary {
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
struct Participant {
    id: String,
    comment_count: u32,
}

/// API error response
#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

/// Query parameters for filtering hidden comments
#[derive(Debug, Serialize, Deserialize)]
struct ShowHiddenQuery {
    show_hidden: Option<bool>,
}

/// Initialize and start the API server with the given VM
pub async fn start_api<S>(vm: VM<S>, port: u16) -> Result<(), Box<dyn std::error::Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm = Arc::new(Mutex::new(vm));

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

    // Combine all routes
    let routes = proposals_route
        .or(comments_route)
        .or(summary_route)
        .with(warp::cors().allow_any_origin())
        .recover(handle_rejection);

    println!("Starting API server on port {}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;

    Ok(())
}

/// Dependency injection helper for the VM
fn with_vm<S>(
    vm: Arc<Mutex<VM<S>>>,
) -> impl Filter<Extract = (Arc<Mutex<VM<S>>>,), Error = Infallible> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    warp::any().map(move || vm.clone())
}

/// Handler for GET /proposals/{id}
async fn get_proposal<S>(id: String, vm: Arc<Mutex<VM<S>>>) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm_lock = vm.lock().await;

    // Load proposal
    let proposal_result = load_proposal_from_governance(&vm_lock, &id);

    match proposal_result {
        Ok(proposal) => {
            // Get vote counts
            let (yes_votes, no_votes, abstain_votes) =
                count_votes(&vm_lock, &id).unwrap_or((0, 0, 0));

            let total_votes = yes_votes + no_votes + abstain_votes;

            // Calculate percentages
            let quorum_percentage = 0.0; // Would need real data from lifecycle
            let threshold_percentage = if total_votes > 0 {
                (yes_votes as f64 / total_votes as f64) * 100.0
            } else {
                0.0
            };

            // Build response
            let response = ProposalResponse {
                id: proposal.id,
                title: "".to_string(), // Would need to fetch from lifecycle
                creator: proposal.creator,
                status: format!("{:?}", proposal.status),
                created_at: proposal.created_at.to_rfc3339(),
                votes: VoteCounts {
                    yes: yes_votes,
                    no: no_votes,
                    abstain: abstain_votes,
                    total: total_votes,
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
    vm: Arc<Mutex<VM<S>>>,
    query: ShowHiddenQuery,
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm_lock = vm.lock().await;

    // Create a null auth context for read-only operations
    let auth_context = None;
    let show_hidden = query.show_hidden.unwrap_or(false);

    // Pass the show_hidden parameter to control visibility of hidden comments
    match crate::governance::comments::fetch_comments_threaded(
        &vm_lock,
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
async fn get_proposal_summary<S>(id: String, vm: Arc<Mutex<VM<S>>>) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm_lock = vm.lock().await;

    // Load proposal and comments
    let proposal_result = load_proposal_from_governance(&vm_lock, &id);
    let comments_result =
        crate::governance::comments::fetch_comments_threaded(&vm_lock, &id, None, false);

    if let (Ok(proposal), Ok(comments)) = (&proposal_result, &comments_result) {
        // Count votes
        let (yes_votes, no_votes, abstain_votes) = count_votes(&vm_lock, &id).unwrap_or((0, 0, 0));

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
            vote_count: total_votes,
            vote_details: VoteCounts {
                yes: yes_votes,
                no: no_votes,
                abstain: abstain_votes,
                total: total_votes,
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
