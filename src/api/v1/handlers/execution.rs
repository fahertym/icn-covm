use warp::{Rejection, Reply};
use std::sync::Arc;
use crate::storage::traits::{Storage, StorageExtensions};
use crate::api::auth::AuthInfo;
use crate::api::error::{not_found, internal_error};
use crate::api::storage::AsyncStorage;
use log::{info, warn, error};
use serde_json::json;

/// Handler for retrieving execution results for a proposal
pub async fn get_execution_results_handler(
    id: String,
    storage: Arc<impl Storage + StorageExtensions + AsyncStorage + Send + Sync>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    info!("Getting execution results for proposal {}", id);
    
    // Check if proposal exists
    let proposal_exists = async {
        // Check if the key exists using contains
        let namespace = "governance";
        let key = format!("proposals/{}", id);
        storage.contains(None, namespace, &key)
    }.await.map_err(|e| {
        error!("Error checking if proposal exists: {}", id);
        internal_error(format!("Error checking proposal: {}", e))
    })?;
    
    if !proposal_exists {
        return Err(not_found(format!("Proposal with id {} not found", id)).into());
    }
    
    // Get execution result
    let execution_result = storage.get_proposal_execution_result(&id).await.map_or_else(
        |e| {
            warn!("No execution result found for proposal {}: {}", id, e);
            serde_json::Value::Null
        },
        |result| {
            // Try to parse the result as JSON for better presentation
            serde_json::from_str(&result).unwrap_or_else(|_| serde_json::Value::String(result))
        }
    );
    
    // Get execution logs if available
    let execution_logs = storage.get_proposal_execution_logs(&id).await.map_or_else(
        |_| None,
        |logs| if logs.is_empty() { None } else { Some(logs) }
    );
    
    // Get basic proposal details
    let proposal = storage.get_proposal(&id).await.map_err(|e| {
        error!("Error retrieving proposal details: {}", e);
        internal_error(format!("Error retrieving proposal details: {}", e))
    })?;
    
    // Return response with execution data
    let response_data = json!({
        "proposal_id": id,
        "title": proposal.title,
        "status": proposal.status,
        "execution_result": execution_result,
        "execution_logs": execution_logs
    });
    
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({
            "status": "success",
            "message": "Retrieved execution results",
            "data": response_data
        })),
        warp::http::StatusCode::OK,
    ))
} 