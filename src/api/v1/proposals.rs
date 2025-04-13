use warp::{Filter, Rejection, Reply};
use crate::storage::traits::{Storage, StorageExtensions};
use crate::api::auth::{with_auth, AuthInfo, require_role};
use crate::api::error::{not_found, bad_request, internal_error};
use crate::api::storage::AsyncStorage;
use crate::api::v1::models::{
    Proposal, CreateProposalRequest, PaginationParams, SortParams,
    Comment, CreateCommentRequest
};
use crate::vm::VM;
use serde_json::json;
use std::sync::Arc;
use std::fmt::Debug;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use log::{info, warn, error};

/// Get all proposal-related API routes
pub fn get_routes<S>(
    storage: Arc<Storage>,
    vm: VM<S>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Create base path for proposal routes
    let base = warp::path("proposals");
    let vm = Arc::new(vm);
    
    // Create filter for injecting dependencies
    let with_storage = move |storage: Arc<Storage>| warp::any().map(move || storage.clone());
    let with_vm = move |vm: Arc<VM<S>>| warp::any().map(move || vm.clone());
    
    // List proposals route
    let list_proposals = base
        .and(warp::get())
        .and(warp::query::<PaginationParams>())
        .and(warp::query::<SortParams>())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(list_proposals_handler);
    
    let get_proposal = base
        .and(warp::path::param::<String>())
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(get_proposal_handler);
    
    let create_proposal = base
        .and(warp::post())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and(require_role("proposals:create"))
        .and_then(create_proposal_handler);
    
    let update_proposal_status = base
        .and(warp::path::param::<String>())
        .and(warp::path("status"))
        .and(warp::put())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and(require_role("proposals:update"))
        .and_then(update_proposal_status_handler);
    
    let vote_on_proposal = base
        .and(warp::path::param::<String>())
        .and(warp::path("vote"))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and(require_role("proposals:vote"))
        .and_then(vote_on_proposal_handler);
    
    let get_proposal_comments = base
        .and(warp::path::param::<String>())
        .and(warp::path("comments"))
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and_then(get_proposal_comments_handler);
    
    let add_proposal_comment = base
        .and(warp::path::param::<String>())
        .and(warp::path("comments"))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and(with_auth())
        .and(require_role("proposals:comment"))
        .and_then(add_proposal_comment_handler);
    
    let execute_proposal = base
        .and(warp::path::param::<String>())
        .and(warp::path("execute"))
        .and(warp::post())
        .and(with_storage(storage.clone()))
        .and(with_vm(vm.clone()))
        .and(with_auth())
        .and(require_role("proposals:execute"))
        .and_then(execute_proposal_handler);
    
    let retry_execution = base
        .and(warp::path::param::<String>())
        .and(warp::path("execute"))
        .and(warp::path("retry"))
        .and(warp::post())
        .and(with_storage(storage.clone()))
        .and(with_vm(vm.clone()))
        .and(with_auth())
        .and(require_role("proposals:retry"))
        .and_then(retry_execution_handler);
    
    list_proposals
        .or(get_proposal)
        .or(create_proposal)
        .or(update_proposal_status)
        .or(vote_on_proposal)
        .or(get_proposal_comments)
        .or(add_proposal_comment)
        .or(execute_proposal)
        .or(retry_execution)
}

/// Filter helper for storage dependency injection
fn with_storage(storage: Arc<Storage>) -> impl Filter<Extract = (Arc<Storage>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || storage.clone())
}

/// Filter helper for VM dependency injection
fn with_vm<S>(vm: Arc<VM<S>>) -> impl Filter<Extract = (Arc<VM<S>>,), Error = std::convert::Infallible> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    warp::any().map(move || vm.clone())
}

// Handler implementations
async fn list_proposals_handler(
    pagination: PaginationParams,
    sort: SortParams,
    storage: Arc<Storage>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);
    
    let proposals = storage.list_proposals().await
        .map_err(|e| internal_error(e.to_string()))?;
    
    // Apply sorting if specified
    let mut proposal_list = proposals;
    if let Some(sort_by) = &sort.sort_by {
        let is_ascending = sort.sort_dir.as_deref() != Some("desc");
        
        // Sort based on the field
        match sort_by.as_str() {
            "title" => {
                if is_ascending {
                    proposal_list.sort_by(|a, b| a.title.cmp(&b.title));
                } else {
                    proposal_list.sort_by(|a, b| b.title.cmp(&a.title));
                }
            },
            "created_at" => {
                if is_ascending {
                    proposal_list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                } else {
                    proposal_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                }
            },
            "status" => {
                if is_ascending {
                    proposal_list.sort_by(|a, b| a.status.cmp(&b.status));
                } else {
                    proposal_list.sort_by(|a, b| b.status.cmp(&a.status));
                }
            },
            _ => {} // Ignore invalid sort fields
        }
    }
    
    // Apply pagination
    let total = proposal_list.len();
    let start = (page - 1) * page_size;
    let end = std::cmp::min(start + page_size, total);
    
    let paged_proposals = if start < total {
        proposal_list[start..end].to_vec()
    } else {
        vec![]
    };
    
    // Convert to API model
    let api_proposals: Vec<Proposal> = paged_proposals.into_iter()
        .map(|p| Proposal {
            id: p.id,
            title: p.title,
            description: p.description,
            created_at: p.created_at,
            updated_at: p.updated_at,
            status: p.status,
            author: p.author,
            votes_for: p.votes_for.unwrap_or(0),
            votes_against: p.votes_against.unwrap_or(0),
            votes_abstain: p.votes_abstain.unwrap_or(0),
            attachments: vec![], // TODO: Load attachments if needed
        })
        .collect();
    
    let response = json!({
        "total": total,
        "page": page,
        "page_size": page_size,
        "proposals": api_proposals
    });
    
    Ok(warp::reply::json(&response))
}

async fn get_proposal_handler(
    id: String,
    storage: Arc<Storage>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    let proposal = storage.get_proposal(&id).await
        .map_err(|_| not_found(format!("Proposal with id {} not found", id)))?;
    
    // Convert to API model
    let api_proposal = Proposal {
        id: proposal.id,
        title: proposal.title,
        description: proposal.description,
        created_at: proposal.created_at,
        updated_at: proposal.updated_at,
        status: proposal.status,
        author: proposal.author,
        votes_for: proposal.votes_for.unwrap_or(0),
        votes_against: proposal.votes_against.unwrap_or(0),
        votes_abstain: proposal.votes_abstain.unwrap_or(0),
        attachments: vec![], // TODO: Load attachments if needed
    };
    
    Ok(warp::reply::json(&api_proposal))
}

async fn create_proposal_handler(
    create_request: CreateProposalRequest,
    storage: Arc<Storage>,
    auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection> {
    // Create new proposal
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    
    let proposal = crate::storage::Proposal {
        id: id.clone(),
        title: create_request.title,
        description: create_request.description,
        created_at: now.clone(),
        updated_at: now,
        status: "DRAFT".to_string(),
        author: auth.user_id.unwrap_or_else(|| "anonymous".to_string()),
        votes_for: Some(0),
        votes_against: Some(0),
        votes_abstain: Some(0),
        attachments: None,
    };
    
    storage.save_proposal(&proposal).await
        .map_err(|e| internal_error(format!("Failed to save proposal: {}", e)))?;
    
    // Save attachments if provided
    if !create_request.attachments.is_empty() {
        for attachment in create_request.attachments {
            let attachment_id = Uuid::new_v4().to_string();
            let attachment = crate::storage::ProposalAttachment {
                id: attachment_id,
                proposal_id: id.clone(),
                name: attachment.name,
                content_type: attachment.content_type,
                url: attachment.url,
                size: attachment.size,
            };
            
            storage.save_proposal_attachment(&attachment).await
                .map_err(|e| internal_error(format!("Failed to save attachment: {}", e)))?;
        }
    }
    
    // Return the created proposal
    let api_proposal = Proposal {
        id: proposal.id,
        title: proposal.title,
        description: proposal.description,
        created_at: proposal.created_at,
        updated_at: proposal.updated_at,
        status: proposal.status,
        author: proposal.author,
        votes_for: 0,
        votes_against: 0,
        votes_abstain: 0,
        attachments: create_request.attachments,
    };
    
    Ok(warp::reply::with_status(
        warp::reply::json(&api_proposal),
        warp::http::StatusCode::CREATED,
    ))
}

async fn update_proposal_status_handler(
    id: String,
    status_update: serde_json::Value,
    storage: Arc<Storage>,
    _auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection> {
    // Extract status from the request
    let status = match status_update.get("status") {
        Some(status) => match status.as_str() {
            Some(s) => s,
            None => return Err(bad_request("Status must be a string".to_string()).into()),
        },
        None => return Err(bad_request("Status field is required".to_string()).into()),
    };
    
    // Validate status
    match status {
        "DRAFT" | "OPEN" | "APPROVED" | "REJECTED" | "CLOSED" => {},
        _ => return Err(bad_request(format!("Invalid status: {}", status)).into()),
    }
    
    // Get existing proposal
    let mut proposal = storage.get_proposal(&id).await
        .map_err(|_| not_found(format!("Proposal with id {} not found", id)))?;
    
    // Update status
    proposal.status = status.to_string();
    proposal.updated_at = Utc::now().to_rfc3339();
    
    // Save updated proposal
    storage.save_proposal(&proposal).await
        .map_err(|e| internal_error(format!("Failed to update proposal: {}", e)))?;
    
    // Convert to API model
    let api_proposal = Proposal {
        id: proposal.id,
        title: proposal.title,
        description: proposal.description,
        created_at: proposal.created_at,
        updated_at: proposal.updated_at,
        status: proposal.status,
        author: proposal.author,
        votes_for: proposal.votes_for.unwrap_or(0),
        votes_against: proposal.votes_against.unwrap_or(0),
        votes_abstain: proposal.votes_abstain.unwrap_or(0),
        attachments: vec![], // TODO: Load attachments if needed
    };
    
    Ok(warp::reply::json(&api_proposal))
}

async fn vote_on_proposal_handler(
    id: String,
    vote: serde_json::Value,
    storage: Arc<Storage>,
    auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection> {
    // Extract vote type from the request
    let vote_type = match vote.get("vote") {
        Some(vote_type) => match vote_type.as_str() {
            Some(v) => v,
            None => return Err(bad_request("Vote must be a string".to_string()).into()),
        },
        None => return Err(bad_request("Vote field is required".to_string()).into()),
    };
    
    // Validate vote type
    match vote_type {
        "FOR" | "AGAINST" | "ABSTAIN" => {},
        _ => return Err(bad_request(format!("Invalid vote type: {}", vote_type)).into()),
    }
    
    // Get existing proposal
    let mut proposal = storage.get_proposal(&id).await
        .map_err(|_| not_found(format!("Proposal with id {} not found", id)))?;
    
    // Check if proposal is open for voting
    if proposal.status != "OPEN" {
        return Err(bad_request("Cannot vote on a proposal that is not open".to_string()).into());
    }
    
    // Get user ID
    let user_id = auth.user_id.ok_or_else(|| 
        bad_request("User ID is required for voting".to_string())
    )?;
    
    // Record vote
    let vote_id = Uuid::new_v4().to_string();
    let vote = crate::storage::Vote {
        id: vote_id,
        proposal_id: id.clone(),
        user_id,
        vote_type: vote_type.to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    
    storage.save_vote(&vote).await
        .map_err(|e| internal_error(format!("Failed to save vote: {}", e)))?;
    
    // Update vote counts
    match vote_type {
        "FOR" => {
            proposal.votes_for = Some(proposal.votes_for.unwrap_or(0) + 1);
        },
        "AGAINST" => {
            proposal.votes_against = Some(proposal.votes_against.unwrap_or(0) + 1);
        },
        "ABSTAIN" => {
            proposal.votes_abstain = Some(proposal.votes_abstain.unwrap_or(0) + 1);
        },
        _ => {}
    }
    
    // Save updated proposal
    storage.save_proposal(&proposal).await
        .map_err(|e| internal_error(format!("Failed to update proposal: {}", e)))?;
    
    // Return success response
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({
            "status": "success",
            "message": format!("Vote '{}' recorded", vote_type)
        })),
        warp::http::StatusCode::OK,
    ))
}

async fn get_proposal_comments_handler(
    id: String,
    pagination: PaginationParams,
    storage: Arc<Storage>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    // Get comments for proposal
    let comments = storage.get_proposal_comments(&id).await
        .map_err(|e| internal_error(format!("Failed to get comments: {}", e)))?;
    
    // Apply pagination
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);
    let total = comments.len();
    let start = (page - 1) * page_size;
    let end = std::cmp::min(start + page_size, total);
    
    let paged_comments = if start < total {
        comments[start..end].to_vec()
    } else {
        vec![]
    };
    
    // Convert to API model
    let api_comments: Vec<Comment> = paged_comments.into_iter()
        .map(|c| Comment {
            id: c.id,
            proposal_id: id.clone(),
            author: c.author,
            content: c.content,
            created_at: c.created_at,
            updated_at: c.updated_at,
            reactions: c.reactions.unwrap_or_default(),
        })
        .collect();
    
    let response = json!({
        "total": total,
        "page": page,
        "page_size": page_size,
        "comments": api_comments
    });
    
    Ok(warp::reply::json(&response))
}

async fn add_proposal_comment_handler(
    id: String,
    comment_request: CreateCommentRequest,
    storage: Arc<Storage>,
    auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection> {
    // Check if proposal exists
    let _ = storage.get_proposal(&id).await
        .map_err(|_| not_found(format!("Proposal with id {} not found", id)))?;
    
    // Get user ID
    let user_id = auth.user_id.ok_or_else(|| 
        bad_request("User ID is required for commenting".to_string())
    )?;
    
    // Create comment
    let comment_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    
    let comment = crate::storage::Comment {
        id: comment_id,
        proposal_id: id,
        author: user_id,
        content: comment_request.content,
        created_at: now.clone(),
        updated_at: now,
        reactions: None,
    };
    
    // Save comment
    storage.save_comment(&comment).await
        .map_err(|e| internal_error(format!("Failed to save comment: {}", e)))?;
    
    // Convert to API model
    let api_comment = Comment {
        id: comment.id,
        proposal_id: comment.proposal_id,
        author: comment.author,
        content: comment.content,
        created_at: comment.created_at,
        updated_at: comment.updated_at,
        reactions: HashMap::new(),
    };
    
    Ok(warp::reply::with_status(
        warp::reply::json(&api_comment),
        warp::http::StatusCode::CREATED,
    ))
}

// New handler to execute a proposal
async fn execute_proposal_handler<S>(
    id: String,
    storage: Arc<Storage>,
    vm: Arc<VM<S>>,
    auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    info!("Executing proposal {}", id);
    
    // Get existing proposal
    let mut proposal = storage.get_proposal(&id).await
        .map_err(|_| {
            error!("Proposal with id {} not found", id);
            not_found(format!("Proposal with id {} not found", id))
        })?;
    
    // Check if proposal is in APPROVED state
    if proposal.status != "APPROVED" {
        warn!("Cannot execute proposal {} that is not in APPROVED state. Current state: {}", id, proposal.status);
        return Err(bad_request(format!("Cannot execute proposal that is not in APPROVED state. Current state: {}", proposal.status)).into());
    }
    
    // Get the DSL logic path (assuming it's stored in the attachments or metadata)
    let logic_path = match storage.get_proposal_logic_path(&id).await {
        Ok(path) => path,
        Err(_) => {
            error!("No logic_path found for proposal {}", id);
            return Err(bad_request("No logic path found for this proposal").into());
        }
    };
    
    // Get the DSL code
    let dsl_code = match storage.get_proposal_logic(&logic_path).await {
        Ok(code) => code,
        Err(e) => {
            error!("Failed to retrieve DSL code for proposal {}: {}", id, e);
            return Err(internal_error(format!("Failed to retrieve DSL code: {}", e)).into());
        }
    };
    
    info!("Executing DSL logic for proposal {}", id);
    
    // Execute the DSL code with the proposal context
    let context = json!({
        "proposal_id": id,
        "creator": proposal.author,
        "votes_for": proposal.votes_for.unwrap_or(0),
        "votes_against": proposal.votes_against.unwrap_or(0),
        "votes_abstain": proposal.votes_abstain.unwrap_or(0),
    });
    
    let execution_result = match vm.execute_dsl(&dsl_code, Some(context)) {
        Ok(result) => {
            info!("Successfully executed proposal {}", id);
            result
        },
        Err(e) => {
            error!("Failed to execute DSL code for proposal {}: {}", id, e);
            return Err(internal_error(format!("Failed to execute proposal logic: {}", e)).into());
        }
    };
    
    // Update proposal with execution result and change status to EXECUTED
    proposal.status = "EXECUTED".to_string();
    proposal.updated_at = Utc::now().to_rfc3339();
    
    // Store the execution result (we'll convert it to a string)
    let result_string = serde_json::to_string(&execution_result)
        .unwrap_or_else(|_| String::from("\"Execution completed, but result couldn't be serialized\""));
    
    // Save execution result with versioning
    let success = true; // Assume success since we reached this point
    let summary = "Proposal executed successfully";
    match storage.save_proposal_execution_result_versioned(&id, &result_string, success, summary).await {
        Ok(version) => info!("Saved execution result version {} for proposal {}", version, id),
        Err(e) => {
            warn!("Failed to save execution result for proposal {}: {}", id, e);
            // We'll continue even if saving the result fails
        }
    }
    
    // Save updated proposal
    storage.save_proposal(&proposal).await
        .map_err(|e| {
            error!("Failed to update proposal {} after execution: {}", id, e);
            internal_error(format!("Failed to update proposal after execution: {}", e))
        })?;
    
    // Get any execution logs if available
    let execution_logs = match storage.get_proposal_execution_logs(&id).await {
        Ok(logs) => Some(logs),
        Err(_) => None,
    };
    
    // Return success response
    let response_data = json!({
        "proposal_id": id,
        "status": "EXECUTED",
        "execution_result": execution_result,
        "execution_logs": execution_logs
    });
    
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({
            "status": "success",
            "message": "Proposal executed successfully",
            "data": response_data
        })),
        warp::http::StatusCode::OK,
    ))
}

// New handler to retry execution of a failed proposal
async fn retry_execution_handler<S>(
    id: String,
    storage: Arc<Storage>,
    vm: Arc<VM<S>>,
    auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    info!("Retrying execution for proposal {}", id);
    
    // Get user identity for logging
    let user_identity = auth.user_id.clone().unwrap_or_else(|| "anonymous".to_string());
    
    // Get the proposal lifecycle from storage
    let proposal_lifecycle_key = format!("governance/proposals/{}/lifecycle", id);
    let mut lifecycle = storage.get_json::<ProposalLifecycle>(None, "governance", &proposal_lifecycle_key)
        .map_err(|e| {
            error!("Failed to load proposal lifecycle for {}: {}", id, e);
            match e {
                StorageError::KeyNotFound { .. } => not_found(format!("Proposal with id {} not found", id)),
                _ => internal_error(format!("Error loading proposal: {}", e))
            }
        })?;
    
    // Try to load execution metadata if necessary
    if lifecycle.execution_status.is_some() && lifecycle.execution_metadata.is_none() {
        if let Err(e) = lifecycle.load_execution_metadata(&*storage) {
            warn!("Failed to load execution metadata for proposal {}: {}", id, e);
            // Continue anyway, it's not critical
        }
    }
    
    // Get existing proposal for API response
    let mut proposal = storage.get_proposal(&id).await
        .map_err(|_| {
            error!("Proposal with id {} not found", id);
            not_found(format!("Proposal with id {} not found", id))
        })?;
    
    // Create a mutable VM to work with
    let mut mutable_vm = (*vm).clone();
    
    // Log the retry attempt
    let timestamp = Utc::now();
    let timestamp_str = timestamp.to_rfc3339();
    let retry_count = lifecycle.execution_metadata.as_ref().map_or(0, |m| m.retry_count);
    
    // Create a log entry for the attempt
    let log_attempt = format!(
        "[{}] RETRY by user:{} | attempting retry #{}", 
        timestamp_str, 
        user_identity,
        retry_count + 1
    );
    
    // Append to execution logs
    let _ = storage.append_proposal_execution_log(&id, &log_attempt).await;
    
    // Attempt to retry execution
    match lifecycle.retry_execution(&mut mutable_vm, None) {
        Ok(result) => {
            info!("Successfully retried execution for proposal {}", id);
            
            // Log successful retry
            let success_log = format!(
                "[{}] RETRY by user:{} | status: success | retry_count: {}", 
                timestamp_str, 
                user_identity,
                retry_count + 1
            );
            let _ = storage.append_proposal_execution_log(&id, &success_log).await;
            
            // Update API proposal model with new execution status
            proposal.status = "EXECUTED".to_string(); // Status remains EXECUTED
            proposal.updated_at = Utc::now().to_rfc3339();
            
            // Save the updated API proposal model
            storage.save_proposal(&proposal).await
                .map_err(|e| {
                    error!("Failed to update proposal {} after execution retry: {}", id, e);
                    internal_error(format!("Failed to update proposal after execution retry: {}", e))
                })?;
            
            // Get execution logs (most recent 5) if available
            let execution_logs = match storage.get_proposal_execution_logs(&id, Some(5)).await {
                Ok(logs) => Some(logs),
                Err(_) => None,
            };
            
            // Get the latest execution result
            let execution_result = match storage.get_latest_execution_result(&id).await {
                Ok(result_str) => serde_json::from_str(&result_str)
                    .unwrap_or_else(|_| json!({"error": "Failed to parse execution result"})),
                Err(_) => json!({"error": "Failed to retrieve execution result"}),
            };
            
            // Return success response
            let response_data = json!({
                "proposal_id": id,
                "status": "EXECUTED",
                "execution_result": execution_result,
                "execution_logs": execution_logs,
                "execution_status": format!("{:?}", result),
                "execution_metadata": lifecycle.execution_metadata,
                "retry_info": {
                    "retry_count": lifecycle.execution_metadata.as_ref().map_or(0, |m| m.retry_count),
                    "last_retry_at": lifecycle.execution_metadata.as_ref().and_then(|m| m.last_retry_at.map(|t| t.to_rfc3339())),
                    "max_retries": crate::governance::proposal_lifecycle::MAX_RETRIES,
                    "cooldown_minutes": crate::governance::proposal_lifecycle::COOLDOWN_DURATION.num_minutes(),
                    "recent_logs": execution_logs
                }
            });
            
            Ok(warp::reply::with_status(
                warp::reply::json(&json!({
                    "status": "success",
                    "message": "Proposal execution retry completed",
                    "data": response_data
                })),
                warp::http::StatusCode::OK,
            ))
        },
        Err(e) => {
            error!("Failed to retry execution for proposal {}: {}", id, e);
            
            // Log failed retry with reason
            let failure_log = format!(
                "[{}] RETRY by user:{} | status: failed | reason: {}", 
                timestamp_str, 
                user_identity,
                e
            );
            let _ = storage.append_proposal_execution_log(&id, &failure_log).await;
            
            // Return error response with last few logs
            let execution_logs = match storage.get_proposal_execution_logs(&id, Some(5)).await {
                Ok(logs) => Some(logs),
                Err(_) => None,
            };
            
            let error_response = json!({
                "status": "error",
                "message": format!("Failed to retry execution: {}", e),
                "retry_info": {
                    "retry_count": lifecycle.execution_metadata.as_ref().map_or(0, |m| m.retry_count),
                    "last_retry_at": lifecycle.execution_metadata.as_ref().and_then(|m| m.last_retry_at.map(|t| t.to_rfc3339())),
                    "max_retries": crate::governance::proposal_lifecycle::MAX_RETRIES,
                    "cooldown_minutes": crate::governance::proposal_lifecycle::COOLDOWN_DURATION.num_minutes(),
                    "recent_logs": execution_logs
                }
            });
            
            // Return error response
            Err(warp::reject::custom(bad_request(format!("Failed to retry execution: {}", e))))
        }
    }
} 