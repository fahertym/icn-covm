use chrono::{DateTime, Utc};
use futures::{FutureExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use warp::{Filter, Reply, Rejection, reject, http::StatusCode};

use crate::api::auth::{AuthInfo, with_auth};
use crate::api::error::{ApiError, bad_request, internal_error, not_found};
use crate::api::storage::AsyncStorage;
use crate::governance::ProposalLifecycle;
use crate::governance::proposal_lifecycle::ProposalState as ProposalStatus;
use crate::response::{ApiResponse, ResponseMeta};
use crate::storage::{Proposal as StorageProposal, ProposalAttachment as StorageAttachment, Vote, Comment as StorageComment};
use crate::storage::errors::StorageError;
use crate::storage::traits::{StorageBackend, StorageExtensions, AsyncStorageExtensions, JsonStorage};
use crate::api::v1::models::{CreateProposalRequest, ProposalAttachment, CreateCommentRequest, PaginationParams as ApiPaginationParams, SortParams as ApiSortParams, VoteCounts, VoteBreakdown, ProposalResponse, ProposalSummary, Participant, VoteType as ApiVoteType, Vote as ApiVoteData, CommentResponse as ApiComment};
use crate::vm::{VM, ExecutionContext};
use crate::storage;

/// Response structure for proposal execution endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResponse {
    pub success: bool,
    pub output: serde_json::Value,
    pub logs: String,
    pub version: u64,
}

/// Get the routes for proposals
pub fn get_routes<S>(
    base: &str,
    storage: Arc<Mutex<S>>,
    vm_arc: Arc<VM<Arc<Mutex<S>>>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    let base_path = warp::path(base.to_string())
        .and(warp::path("proposals"));

    // List proposals
    let list = base_path.clone()
        .and(warp::get())
        .and(warp::path::end())
        .and(warp::query::<ApiPaginationParams>())
        .and(warp::query::<ApiSortParams>())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(list_proposals_handler);

    // Get proposal by ID
    let get = base_path.clone()
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(get_proposal_handler);

    // Create proposal
    let create = base_path.clone()
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(create_proposal_handler);

    // Update proposal status
    let update_status = base_path.clone()
        .and(warp::patch())
        .and(warp::path::param::<String>())
        .and(warp::path("status"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(update_proposal_status_handler);

    // Vote on proposal
    let vote = base_path.clone()
        .and(warp::post())
        .and(warp::path::param::<String>())
        .and(warp::path("votes"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(vote_on_proposal_handler);

    // Get comments for a proposal
    let get_comments = base_path.clone()
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path("comments"))
        .and(warp::path::end())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(get_proposal_comments_handler);

    // Add comment to a proposal
    let add_comment = base_path.clone()
        .and(warp::post())
        .and(warp::path::param::<String>())
        .and(warp::path("comments"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(add_proposal_comment_handler);

    // Execute proposal
    let execute = base_path.clone()
        .and(warp::post())
        .and(warp::path::param::<String>())
        .and(warp::path("execute"))
        .and(warp::path::end())
        .and(with_storage(storage.clone()))
        .and(with_vm(vm_arc.clone()))
        .and(with_auth())
        .and_then(execute_proposal_handler);

    // Retry execution
    let retry = base_path.clone()
        .and(warp::post())
        .and(warp::path::param::<String>())
        .and(warp::path("retry"))
        .and(warp::path::end())
        .and(with_storage(storage.clone()))
        .and(with_vm(vm_arc.clone()))
        .and(with_auth())
        .and_then(retry_execution_handler);

    // Use boxed filter to combine these routes
    list
        .or(get)
        .or(create)
        .or(update_status)
        .or(vote)
        .or(get_comments)
        .or(add_comment)
        .or(execute)
        .or(retry)
        .boxed()
}

/// Helper function to pass storage to handlers
fn with_storage<S>(
    storage: Arc<Mutex<S>>,
) -> impl Filter<Extract = (Arc<Mutex<S>>,), Error = Infallible> + Clone
where
    S: Send + Sync + 'static,
{
    warp::any().map(move || storage.clone())
}

/// Helper function to pass VM to handlers
fn with_vm<V>(
    vm: Arc<V>,
) -> impl Filter<Extract = (Arc<V>,), Error = Infallible> + Clone
where
    V: Send + Sync + 'static,
{
    warp::any().map(move || vm.clone())
}

// Helper function to convert storage vote type to API vote type
fn storage_vote_type_to_api(vote_type: &str) -> Option<ApiVoteType> {
    match vote_type.to_lowercase().as_str() {
        "yes" => Some(ApiVoteType::Yes),
        "no" => Some(ApiVoteType::No),
        "abstain" => Some(ApiVoteType::Abstain),
        _ => None,
    }
}

// Helper method to get votes since get_votes doesn't exist directly on the storage lock
async fn get_votes<S>(storage_lock: &S, proposal_id: &str) -> Result<Vec<Vote>, StorageError>
where 
    S: AsyncStorage + Send + Sync
{
    storage_lock.get_proposal_votes(proposal_id).await
}

// Helper to safely get execution result from proposal
fn get_execution_result(proposal: &StorageProposal) -> Option<String> {
    // StorageProposal doesn't have execution_result directly, so we check if it needs to be computed
    // or retrieved in a different way based on the application logic
    None // Default implementation - replace with actual logic if available
}

// Helper functions
fn proposal_to_response(proposal: &StorageProposal, vote_counts: VoteCounts, execution_result: Option<String>) -> ProposalResponse {
    ProposalResponse {
        id: proposal.id.clone(),
        title: proposal.title.clone(),
        creator: proposal.author.clone(),
        status: proposal.status.clone(),
        created_at: proposal.created_at.clone(), // Already a string
        votes: vote_counts,
        quorum_percentage: 0.0, // Placeholder
        threshold_percentage: 0.0, // Placeholder
        execution_result: execution_result, // Pass execution result in
    }
}

fn comment_to_api_comment(comment: &StorageComment) -> ApiComment {
    // Convert reactions HashMap from <String, i32> to <String, u32>
    let reactions = comment.reactions.clone().unwrap_or_default()
        .into_iter()
        .map(|(k, v)| (k, v as u32))
        .collect();

    ApiComment {
        id: comment.id.clone(),
        author: comment.author.clone(),
        timestamp: comment.created_at.clone(),
        content: comment.content.clone(),
        reply_to: None, // Storage comment doesn't have this field
        tags: Vec::new(), // Storage comment doesn't have this field
        reactions,
        hidden: false, // Storage comment doesn't have this field
        edit_count: 0, // Storage comment doesn't have this field
    }
}

// Handlers
async fn list_proposals_handler<S>(
    pagination: ApiPaginationParams,
    sort: ApiSortParams,
    storage: Arc<Mutex<S>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Listing proposals with pagination: {:?}, sort: {:?}", pagination, sort);
    let storage_lock = storage.lock().await;
    
    // Use the basic list_proposals method with no parameters since the extended version 
    // with pagination and sorting parameters is not available
    let proposals_result = storage_lock.list_proposals().await;

    match proposals_result {
        Ok(proposals) => {
            // Sort and paginate the results manually
            let summaries: Vec<ProposalSummary> = vec![];
            // TODO: Convert proposals to summaries
            
            // Just return a plain response for now
            Ok(warp::reply::json(&summaries))
        },
        Err(e) => {
            error!("Failed to list proposals: {}", e);
            Err(reject::custom(internal_error(&format!("Failed to list proposals: {}", e))))
        }
    }
}

async fn get_proposal_handler<S>(
    id: String,
    storage: Arc<Mutex<S>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Getting proposal with ID: {}", id);
    let storage_lock = storage.lock().await;
    let proposal = match storage_lock.get_proposal(&id).await {
        Ok(p) => p,
        Err(StorageError::NotFound { .. }) => {
            warn!("Proposal {} not found", id);
            return Err(reject::custom(not_found(&format!("Proposal {} not found", id))));
        },
        Err(e) => {
            error!("Failed to load proposal {}: {}", id, e);
            return Err(reject::custom(internal_error(&format!("Failed to load proposal: {}", e))));
        }
    };

    let votes = match get_votes(&*storage_lock, &id).await {
        Ok(v) => v,
        Err(e) => {
            warn!("Could not load votes for proposal {}: {}. Returning 0 counts.", id, e);
            vec![]
        }
    };

    let mut yes_votes = 0u32;
    let mut no_votes = 0u32;
    let mut abstain_votes = 0u32;
    for vote in votes {
        if let Some(vote_type) = storage_vote_type_to_api(&vote.vote_type) {
            match vote_type {
                ApiVoteType::Yes => yes_votes += 1,
                ApiVoteType::No => no_votes += 1,
                ApiVoteType::Abstain => abstain_votes += 1,
            }
        } else {
            warn!("Found invalid vote type '{}' in storage for proposal {}", vote.vote_type, id);
        }
    }

    let vote_counts = VoteCounts { 
        vote_count: yes_votes + no_votes + abstain_votes, 
        breakdown: VoteBreakdown {
            yes: yes_votes,
            no: no_votes,
            abstain: abstain_votes,
            total: yes_votes + no_votes + abstain_votes
        }
    };
    let execution_result = get_execution_result(&proposal);
    let response = proposal_to_response(&proposal, vote_counts, execution_result);
    Ok(warp::reply::json(&response))
}

async fn create_proposal_handler<S>(
    create_request: CreateProposalRequest,
    storage: Arc<Mutex<S>>,
    auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Creating new proposal titled: {}", create_request.title);
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339(); // Use RFC3339 string format consistent with StorageProposal
    let attachments_req = create_request.attachments; // No need for unwrap_or_default if request requires it
    let proposal = StorageProposal {
        id: id.clone(),
        title: create_request.title,
        description: create_request.details,
        author: auth.user_id.clone(),
        // Serialize enum to string for storage
        status: serde_json::to_string(&ProposalStatus::Draft).unwrap_or_else(|_| "Draft".to_string()),
        created_at: now.clone(),
        updated_at: now.clone(), // Use RFC3339 string
        // StorageProposal fields based on src/storage/mod.rs
        votes_for: Some(0),
        votes_against: Some(0),
        votes_abstain: Some(0),
        attachments: None, // Initialize attachments as None initially
    };

    let mut storage_lock = storage.lock().await;
    if let Err(e) = storage_lock.save_proposal(&proposal).await {
        error!("Failed to save new proposal {}: {}", id, e);
        return Err(reject::custom(internal_error(&format!("Failed to save proposal: {}", e))));
    }

    let mut saved_attachments: Vec<StorageAttachment> = Vec::new();
    for attachment_req in attachments_req {
        // attachment_req is already models::ProposalAttachment (aliased as ApiAttachment)
        let storage_attachment = StorageAttachment {
            id: Uuid::new_v4().to_string(),
            proposal_id: proposal.id.clone(),
            name: attachment_req.name, // Use field from ApiAttachment
            content_type: attachment_req.mime_type, // Use field from ApiAttachment
            url: "".to_string(), // Placeholder URL - needs actual generation
            size: attachment_req.content.len() as i64, // Placeholder size - calculate actual size
        };
        // Here we would save the actual attachment content (attachment_req.content)
        // storage_lock.save_attachment_content(&storage_attachment.id, &attachment_req.content).await?;
        // And then save the metadata
        if let Err(e) = storage_lock.save_proposal_attachment(&storage_attachment).await {
            error!("Failed to save attachment for proposal {}: {}", id, e);
            // Consider rolling back proposal save or deleting already saved attachments
            return Err(reject::custom(internal_error(&format!("Failed to save attachment: {}", e))));
        }
        saved_attachments.push(storage_attachment);
    }

    let vote_counts = VoteCounts { 
        vote_count: 0, 
        breakdown: VoteBreakdown {
            yes: 0,
            no: 0,
            abstain: 0,
            total: 0
        }
    };
    // Pass None for execution_result as it's not set for new proposals
    let response = proposal_to_response(&proposal, vote_counts, None);
    info!("Proposal {} created successfully", response.id);
    Ok(warp::reply::with_status(warp::reply::json(&response), StatusCode::CREATED))
}

async fn update_proposal_status_handler<S>(
    id: String,
    status_update: serde_json::Value,
    storage: Arc<Mutex<S>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    let status_str = status_update["status"]
        .as_str()
        .ok_or_else(|| reject::custom(bad_request("Missing or invalid 'status' field")))?;

    // Validate the input string can be deserialized into the enum, but store the string
    let _status_enum: ProposalStatus = serde_json::from_str(&format!("\"{}\"", status_str))
        .map_err(|_| reject::custom(bad_request(&format!("Invalid proposal status: {}", status_str))))?;

    info!("Updating status for proposal {} to {}", id, status_str);

    let mut storage_lock = storage.lock().await;
    let mut proposal = match storage_lock.get_proposal(&id).await {
        Ok(p) => p,
        Err(StorageError::NotFound { .. }) => {
            warn!("Proposal {} not found for status update", id);
            return Err(reject::custom(not_found(&format!(
                "Proposal {} not found",
                id
            ))));
        }
        Err(e) => {
            error!("Failed to load proposal {} for status update: {}", id, e);
            return Err(reject::custom(internal_error(&format!(
                "Failed to load proposal: {}",
                e
            ))));
        }
    };

    proposal.status = status_str.to_string(); // Assign the validated string
    proposal.updated_at = Utc::now().to_rfc3339(); // Use RFC3339 string

    if let Err(e) = storage_lock.save_proposal(&proposal).await {
        error!("Failed to save updated proposal {}: {}", id, e);
        return Err(reject::custom(internal_error(&format!(
            "Failed to update proposal: {}",
            e
        ))));
    }

    let votes = match get_votes(&*storage_lock, &id).await { Ok(v) => v, Err(_) => vec![] };
    let mut yes_votes = 0u32;
    let mut no_votes = 0u32;
    let mut abstain_votes = 0u32;
    for vote in votes {
        // Use string comparison for vote types
        match vote.vote_type.to_lowercase().as_str() {
            "yes" => yes_votes += 1,
            "no" => no_votes += 1,
            "abstain" => abstain_votes += 1,
            invalid_type => warn!(
                "Found invalid vote type '{}' in storage for proposal {}",
                invalid_type,
                id
            ),
        }
    }
    let vote_counts = VoteCounts { 
        vote_count: yes_votes + no_votes + abstain_votes, 
        breakdown: VoteBreakdown {
            yes: yes_votes,
            no: no_votes,
            abstain: abstain_votes,
            total: yes_votes + no_votes + abstain_votes
        }
    };
    let execution_result = get_execution_result(&proposal);
    let response = proposal_to_response(&proposal, vote_counts, execution_result);
    info!("Proposal {} status updated successfully", response.id);
    Ok(warp::reply::json(&response))
}

async fn vote_on_proposal_handler<S>(
    id: String,
    vote_req: serde_json::Value,
    storage: Arc<Mutex<S>>,
    auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    let voter_id = auth.user_id;
    let vote_type_str = vote_req["vote"]
        .as_str()
        .ok_or_else(|| reject::custom(bad_request("Missing or invalid 'vote' field")))?;

    // Validate vote_type_str directly
    let lower_vote_type = vote_type_str.to_lowercase();
    if !["yes", "no", "abstain"].contains(&lower_vote_type.as_str()) {
        return Err(reject::custom(bad_request(&format!(
            "Invalid vote type: {}",
            vote_type_str
        ))));
    }

    info!(
        "Processing vote ({}) from {} for proposal {}",
        vote_type_str, voter_id, id
    );
    let mut storage_lock = storage.lock().await;
    let proposal = match storage_lock.get_proposal(&id).await {
        Ok(p) => p,
        Err(StorageError::NotFound { .. }) => {
            warn!("Proposal {} not found for voting", id);
            return Err(reject::custom(not_found(&format!(
                "Proposal {} not found",
                id
            ))));
        }
        Err(e) => {
            error!("Failed to load proposal {} for voting: {}", id, e);
            return Err(reject::custom(internal_error(&format!(
                "Failed to load proposal: {}",
                e
            ))));
        }
    };

    // Compare proposal.status (String) with "Voting"
    let voting_status_str = serde_json::to_string(&ProposalStatus::Voting).unwrap_or_else(|_| "Voting".to_string());
    if proposal.status != voting_status_str {
        warn!(
            "Attempted to vote on proposal {} with status {}",
            id,
            proposal.status
        );
        return Err(reject::custom(bad_request(&format!(
            "Proposal is not currently open for voting (status: {})",
            proposal.status
        ))));
    }

    let vote_record = Vote {
        id: Uuid::new_v4().to_string(), // Generate ID for the vote
        proposal_id: id.clone(),
        user_id: voter_id.clone(),
        vote_type: lower_vote_type, // Use validated lowercase string
        created_at: Utc::now().to_rfc3339(),
        // StorageVote doesn't have metadata
    };

    if let Err(e) = storage_lock.save_vote(&vote_record).await {
        error!("Failed to save vote for proposal {}: {}", id, e);
        return Err(reject::custom(internal_error(&format!(
            "Failed to save vote: {}",
            e
        ))));
    }

    let votes = match get_votes(&*storage_lock, &id).await {
        Ok(v) => v,
        Err(_) => vec![],
    };
    let mut yes_votes = 0u32;
    let mut no_votes = 0u32;
    let mut abstain_votes = 0u32;
    for vote in votes {
        // Use string comparison for vote types
        match vote.vote_type.to_lowercase().as_str() {
            "yes" => yes_votes += 1,
            "no" => no_votes += 1,
            "abstain" => abstain_votes += 1,
            invalid_type => warn!(
                "Found invalid vote type '{}' in storage for proposal {}",
                invalid_type,
                id
            ),
        }
    }
    let vote_counts = VoteCounts { 
        vote_count: yes_votes + no_votes + abstain_votes, 
        breakdown: VoteBreakdown {
            yes: yes_votes,
            no: no_votes,
            abstain: abstain_votes,
            total: yes_votes + no_votes + abstain_votes
        }
    };
    // Fetch proposal again to get potentially updated state (though voting shouldn't change it directly)
    let updated_proposal = match storage_lock.get_proposal(&id).await {
        Ok(p) => p,
        Err(_) => proposal, // Fallback to original if fetch fails
    };
    let execution_result = get_execution_result(&updated_proposal);
    let response = proposal_to_response(&updated_proposal, vote_counts, execution_result);
    info!("Vote recorded successfully for proposal {}", response.id);
    Ok(warp::reply::json(&response))
}

async fn get_proposal_comments_handler<S>(
    id: String,
    storage: Arc<Mutex<S>>,
    auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Getting comments for proposal {}", id);
    let storage_lock = storage.lock().await;
    let all_comments = match storage_lock.get_proposal_comments(&id).await {
         Ok(c) => c,
         Err(e) => {
             error!("Failed to get comments for proposal {}: {}", id, e);
             return Err(reject::custom(internal_error(&format!("Failed to get comments: {}", e))));
         }
    };

    // Default pagination parameters
    let page: u64 = 1;
    let page_size: u64 = 10;
    let total_items = all_comments.len() as u64;
    let start = ((page.saturating_sub(1)) * page_size) as usize;
    let end = start.saturating_add(page_size as usize).min(all_comments.len());

    let paginated_api_comments = all_comments
        .get(start..end)
        .unwrap_or_default()
        .iter()
        .map(|c| comment_to_api_comment(c))
        .collect::<Vec<_>>();

    let response = ApiResponse {
        status: "success".to_string(),
        message: "Comments retrieved successfully".to_string(),
        data: paginated_api_comments,
        meta: Some(ResponseMeta {
            total: total_items,
            page: page,
            per_page: page_size,
        }),
    };
    Ok(warp::reply::json(&response))
}

async fn add_proposal_comment_handler<S>(
    id: String,
    comment_request: CreateCommentRequest,
    storage: Arc<Mutex<S>>,
    auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Adding comment by {} to proposal {}", auth.user_id, id);
    let mut storage_lock = storage.lock().await;
    if let Err(e) = storage_lock.get_proposal(&id).await {
        match e {
             StorageError::NotFound { .. } => {
                 warn!("Proposal {} not found for adding comment", id);
                 return Err(reject::custom(not_found(&format!("Proposal {} not found", id))));
             },
             _ => {
                 error!("Failed to load proposal {} for adding comment: {}", id, e);
                 return Err(reject::custom(internal_error(&format!("Failed to load proposal: {}", e))));
             }
         }
    }

    let now = Utc::now();
    let comment = StorageComment {
        id: Uuid::new_v4().to_string(),
        proposal_id: id.clone(),
        author: auth.user_id.clone(),
        content: comment_request.content,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        reactions: Some(HashMap::new()),
    };

    if let Err(e) = storage_lock.save_comment(&comment).await {
        error!("Failed to save comment for proposal {}: {}", id, e);
        return Err(reject::custom(internal_error(&format!("Failed to save comment: {}", e))));
    }

    let response_comment = comment_to_api_comment(&comment);
    info!("Comment added successfully to proposal {}", id);
    Ok(warp::reply::with_status(warp::reply::json(&response_comment), StatusCode::CREATED))
}

async fn execute_proposal_handler<S>(
    id: String,
    storage: Arc<Mutex<S>>,
    vm_arc: Arc<VM<Arc<Mutex<S>>>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Executing proposal {}", id);
    let mut storage_lock = storage.lock().await;
    let mut proposal = match storage_lock.get_proposal(&id).await {
        Ok(p) => p,
        Err(StorageError::NotFound { .. }) => {
             warn!("Proposal {} not found for execution", id);
            return Err(reject::custom(not_found(&format!("Proposal {} not found", id))));
        },
        Err(e) => {
            error!("Failed to load proposal {} for execution: {}", id, e);
            return Err(reject::custom(internal_error(&format!("Failed to load proposal for execution: {}", e))));
        }
    };

    // Instead of looking for "Approved", check for a status that should allow execution
    let expected_status = "Active"; // or whatever status indicates a proposal can be executed
    if proposal.status != expected_status {
        warn!(
            "Attempted to execute proposal {} with status {}",
            id,
            proposal.status
        );
        return Err(reject::custom(bad_request(&format!(
            "Cannot execute proposal that is not in {} state. Current state: {}",
            expected_status, proposal.status
        ))));
    }

    let proposal_logic_path = storage_lock.get_proposal_logic_path(&id).await.map_err(|e| {
        error!("Failed to retrieve proposal logic path for {}: {}", id, e);
        reject::custom(internal_error(&format!("Failed to retrieve proposal logic path: {}", e)))
    })?;

    let proposal_logic = storage_lock.get_proposal_logic(&proposal_logic_path).await.map_err(|e| {
        error!("Failed to retrieve DSL code for proposal {}: {}", id, e);
        reject::custom(internal_error(&format!("Failed to retrieve DSL code: {}", e)))
    })?;

    let dsl_code = proposal_logic.to_string();

    let execution_context = ExecutionContext {
        proposal_id: Some(id.clone()),
        caller: None,
    };

    drop(storage_lock);
    
    // Use VM directly since Arc<VM<...>> doesn't have a lock method
    let execution_result = vm_arc.execute_dsl(&dsl_code, execution_context);
    
    // Get storage lock again to update the proposal
    let mut storage_lock = storage.lock().await;
    
    // Update proposal status to Executed
    let mut proposal = match storage_lock.get_proposal(&id).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to retrieve proposal after execution: {}", e);
            let response = ExecutionResponse {
                success: true,
                output: execution_result,
                logs: "Execution succeeded but failed to update proposal".to_string(),
                version: 0,
            };
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                StatusCode::INTERNAL_SERVER_ERROR
            ));
        }
    };
    
    proposal.status = "Executed".to_string();
    proposal.updated_at = Utc::now().to_rfc3339();
    
    if let Err(e) = storage_lock.save_proposal(&proposal).await {
        error!("Failed to update proposal status after execution: {}", e);
    }
    
    // Manually save execution result without awaiting
    let version = match storage_lock.save_proposal_execution_result_versioned(
        &id, 
        &serde_json::to_string(&execution_result).unwrap_or_default(), 
        true, 
        "Proposal executed successfully"
    ) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to save execution result: {}", e);
            0
        }
    };
    
    // Create success response
    let response = ExecutionResponse {
        success: true,
        output: execution_result,
        logs: "Execution succeeded".to_string(),
        version,
    };
    
    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        StatusCode::INTERNAL_SERVER_ERROR
    ))
}

async fn retry_execution_handler<S>(
    id: String,
    storage: Arc<Mutex<S>>,
    vm_arc: Arc<VM<Arc<Mutex<S>>>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Retrying execution for proposal {}", id);
    let proposal_lifecycle_key = format!("governance/proposals/{}/lifecycle", id);
    let mut storage_lock = storage.lock().await;

    // Fetch the proposal lifecycle
    let mut lifecycle = storage_lock.get_json::<ProposalLifecycle>(None, "governance", &proposal_lifecycle_key).await
        .map_err(|e| {
            error!("Error loading proposal lifecycle for retry on {}: {}", id, e);
            reject::custom(internal_error(&format!("Error loading proposal lifecycle: {}", e)))
        })?
        .ok_or_else(|| {
            warn!("Proposal lifecycle not found for retry attempt on proposal {}", id);
            reject::custom(not_found(&format!("Proposal lifecycle not found for proposal {}", id)))
        })?;

    // Get existing proposal for context first
    let mut proposal = match storage_lock.get_proposal(&id).await {
        Ok(p) => p,
        Err(StorageError::NotFound { .. }) => {
             warn!("Proposal {} not found during retry attempt", id);
             return Err(reject::custom(not_found(&format!("Proposal {} not found", id))));
        },
        Err(e) => {
             error!("Failed to load proposal {} during retry attempt: {}", id, e);
             return Err(reject::custom(internal_error(&format!("Failed to load proposal during retry: {}", e))));
         }
    };

    // Compare proposal.status (String) with "Failed"
    let failed_status_str = serde_json::to_string(&ProposalStatus::Failed).unwrap_or_else(|_| "Failed".to_string());
    // Note: ProposalState::Failed comparison logic needs separate review
    if proposal.status != failed_status_str { // Basic check on storage string status
         warn!("Attempted to retry proposal {} which is not in Failed state ({})", id, proposal.status);
         return Err(reject::custom(bad_request("Proposal is not in a failed state, cannot retry.")));
    }

    let log_attempt = format!("Execution retry attempted at {} by user {}", Utc::now(), _auth.user_id);
    let _ = storage_lock.append_proposal_execution_log(&id, &log_attempt).await
        .map_err(|e| {
            warn!("Failed to log retry attempt for proposal {}: {}", id, e);
        });

    drop(storage_lock);
    let retry_result = lifecycle.retry_execution(vm_arc.clone(), None).await;
    let mut storage_lock = storage.lock().await;

    match retry_result {
        Ok(_) => {
            info!("Successfully retried execution for proposal {}", id);
            // Assign string representation of enum
            proposal.status = serde_json::to_string(&ProposalStatus::Executed).unwrap_or_else(|_| "Executed".to_string());
            proposal.updated_at = Utc::now().to_rfc3339(); // Use RFC3339 string

            let (execution_result_str, version) = match storage_lock.get_latest_execution_result(&id).await {
                Ok(Some((result_str, v))) => (result_str, v),
                Ok(None) => {
                    warn!("No execution result found after successful retry for {}", id);
                    ("{\"status\": \"No execution result found after successful retry\"}".to_string(), 0)
                },
                Err(e) => {
                    error!("Failed to retrieve latest execution result after retry for {}: {}", id, e);
                    ("{\"error\": \"Failed to retrieve execution result\"}".to_string(), 0)
                }
            };
             let execution_result_json = serde_json::from_str(&execution_result_str).unwrap_or_else(|e| {
                 error!("Failed to parse latest execution result string after retry for {}: {}", id, e);
                 json!({"error": "Failed to parse execution result"})
             });

            let execution_logs = match storage_lock.get_proposal_execution_logs(&id).await {
                Ok(logs) => logs,
                Err(e) => {
                     warn!("Failed to retrieve execution logs for proposal {}: {}", id, e);
                    "Failed to retrieve execution logs".to_string()
                 }
            };

            let response = ExecutionResponse {
                success: true,
                output: execution_result_json,
                logs: execution_logs,
                version: version,
            };
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                StatusCode::OK
            ))
        }
        Err(e) => {
            error!("Execution retry failed for proposal {}: {}", id, e);
            // Assign string representation of enum
            proposal.status = serde_json::to_string(&ProposalStatus::Failed).unwrap_or_else(|_| "Failed".to_string());
            proposal.updated_at = Utc::now().to_rfc3339(); // Use RFC3339 string
            let error_summary = format!("Execution retry failed: {}", e);

            let success = false;
            let version = match storage_lock.save_proposal_execution_result_versioned(&id, &error_summary, success, &error_summary).await {
                Ok(v) => v,
                Err(log_err) => {
                    warn!("Failed to save execution retry failure result for proposal {}: {}", id, log_err);
                    0
                }
            };

            if let Err(e) = storage_lock.save_proposal(&proposal).await {
                error!(
                    "Failed to update proposal {} after execution retry failure: {}",
                    id,
                    e
                );
                return Err(reject::custom(internal_error(&format!(
                    "Failed to update proposal after execution retry failure: {}",
                    e
                ))));
            }

            let execution_logs = match storage_lock.get_proposal_execution_logs(&id).await {
                Ok(logs) => logs,
                 Err(e) => {
                     warn!("Failed to retrieve execution logs for proposal {}: {}", id, e);
                    "Failed to retrieve execution logs".to_string()
                 }
            };
             let (execution_result_str, version) = match storage_lock.get_latest_execution_result(&id).await {
                Ok(Some((result_str, v))) => (result_str, v),
                Ok(None) => {
                    warn!("No execution result found after retry failure for {}", id);
                    ("{\"status\": \"No execution result found after retry failure\"}".to_string(), version)
                },
                Err(e) => {
                    error!("Failed to retrieve latest execution result after retry failure for {}: {}", id, e);
                    ("{\"error\": \"Failed to retrieve execution result\"}".to_string(), version)
                }
            };
             let execution_result_json = serde_json::from_str(&execution_result_str).unwrap_or_else(|e| {
                 error!("Failed to parse latest execution result string after retry failure for {}: {}", id, e);
                 json!({"error": "Failed to parse execution result"})
             });

            let response = ExecutionResponse {
                success: false,
                output: execution_result_json,
                logs: execution_logs,
                version: version,
            };
           Ok(warp::reply::with_status(
               warp::reply::json(&response),
               StatusCode::INTERNAL_SERVER_ERROR
           ))
        }
    }
}

fn paginate_proposals(
    proposals: Vec<StorageProposal>,
    page: u64,
    page_size: u64,
) -> (Vec<StorageProposal>, usize, u64, u64) {
    let total = proposals.len();
    let page_count = ((total as u64) + page_size - 1) / page_size; // Ceiling division
    let start = ((page.saturating_sub(1)) * page_size) as usize;
    let end = start.saturating_add(page_size as usize).min(total);
    let paginated = proposals.into_iter().skip(start).take(end - start).collect();
    
    (paginated, total, page, page_count)
} 