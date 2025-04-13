use warp::{Filter, Rejection, Reply};
use std::fmt::Debug;
use std::sync::Arc;
use crate::storage::traits::{Storage, StorageExtensions};
use crate::response::{ApiResponse, ResponseMeta};
use crate::models::ProposalId;
use crate::api::v1::models::ExecutionResult;
use crate::api::auth::{with_auth, AuthInfo};
use crate::api::error::{not_found, internal_error};
use crate::api::storage::AsyncStorage;
use serde::{Deserialize, Serialize};
use log::{info, warn, error};
use serde_json::json;

/// Query parameters for execution result requests
#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionResultQuery {
    /// Optional version parameter
    pub version: Option<u64>,
}

/// Returns routes for execution results
pub fn execution_result_routes<S>(
    storage: Arc<S>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: Storage + StorageExtensions + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_filter = warp::any().map(move || storage.clone());
    
    // GET /proposals/:id/execution
    let get_execution_result = warp::path!("proposals" / ProposalId / "execution")
        .and(warp::get())
        .and(warp::query())
        .and(storage_filter.clone())
        .and(with_auth())
        .and_then(crate::api::v1::handlers::execution::get_execution_results_handler);
    
    // GET /proposals/:id/execution/versions
    let list_execution_versions = warp::path!("proposals" / ProposalId / "execution" / "versions")
        .and(warp::get())
        .and(storage_filter.clone())
        .and(with_auth())
        .and_then(crate::api::v1::handlers::execution::list_execution_versions_handler);
    
    get_execution_result.or(list_execution_versions)
}

/// Handler to get an execution result for a proposal, defaulting to the latest version
async fn get_execution_result_handler<S>(
    proposal_id: ProposalId,
    query: Option<ExecutionResultQuery>,
    storage: Arc<S>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Getting execution results for proposal {}", proposal_id);
    
    // Check if proposal exists
    let proposal_key = format!("proposals/{}", proposal_id);
    let namespace = "governance";
    
    let proposal_exists = storage.contains(None, namespace, &proposal_key)
        .map_err(|e| {
            error!("Error checking if proposal exists: {}", proposal_id);
            internal_error(format!("Error checking proposal: {}", e))
        })?;
    
    if !proposal_exists {
        return Err(not_found(format!("Proposal with id {} not found", proposal_id)).into());
    }
    
    let version = query.and_then(|q| q.version);
    
    // Get execution result based on version
    let execution_result = if let Some(version) = version {
        // Get version-specific result (not implemented in AsyncStorage yet)
        match storage.get_proposal_execution_result(&proposal_id.to_string()).await {
            Ok(result) => {
                // Try to parse the result as JSON for better presentation
                serde_json::from_str(&result).unwrap_or_else(|_| json!(result))
            },
            Err(e) => {
                warn!("No execution result found for proposal {}: {}", proposal_id, e);
                return Err(not_found(format!("No execution result found for proposal {}", proposal_id)).into());
            }
        }
    } else {
        // Get latest result
        match storage.get_proposal_execution_result(&proposal_id.to_string()).await {
            Ok(result) => {
                // Try to parse the result as JSON for better presentation
                serde_json::from_str(&result).unwrap_or_else(|_| json!(result))
            },
            Err(e) => {
                warn!("No execution result found for proposal {}: {}", proposal_id, e);
                return Err(not_found(format!("No execution result found for proposal {}", proposal_id)).into());
            }
        }
    };
    
    // Get execution logs if available
    let execution_logs = match storage.get_proposal_execution_logs(&proposal_id.to_string()).await {
        Ok(logs) => if logs.is_empty() { None } else { Some(logs) },
        Err(_) => None,
    };
    
    // Get basic proposal details
    let proposal = match storage.get_proposal(&proposal_id.to_string()).await {
        Ok(p) => p,
        Err(e) => {
            error!("Error retrieving proposal details: {}", e);
            return Err(internal_error(format!("Error retrieving proposal details: {}", e)).into());
        }
    };
    
    // Return response with execution data
    let response_data = json!({
        "proposal_id": proposal_id.to_string(),
        "title": proposal.title,
        "status": proposal.status,
        "execution_result": execution_result,
        "execution_logs": execution_logs,
        "version": version
    });
    
    Ok(warp::reply::json(&ApiResponse::success("Retrieved execution results", response_data)))
}

/// Handler to get a specific version of an execution result
async fn get_execution_result_version_handler<S>(
    proposal_id: ProposalId,
    version: u64,
    storage: Arc<S>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Getting execution results version {} for proposal {}", version, proposal_id);
    
    // Check if proposal exists
    let proposal_exists = storage.get_proposal(&proposal_id.to_string()).await.is_ok();
    
    if !proposal_exists {
        return Err(not_found(format!("Proposal with id {} not found", proposal_id)).into());
    }
    
    // Get execution result - currently limited by AsyncStorage implementation
    // This would ideally use a version-specific method
    let execution_result = match storage.get_proposal_execution_result(&proposal_id.to_string()).await {
        Ok(result) => {
            // Try to parse the result as JSON for better presentation
            serde_json::from_str(&result).unwrap_or_else(|_| json!(result))
        },
        Err(e) => {
            warn!("No execution result found for proposal {}: {}", proposal_id, e);
            return Err(not_found(format!("No execution result found for proposal {}", proposal_id)).into());
        }
    };
    
    // Get execution logs if available
    let execution_logs = match storage.get_proposal_execution_logs(&proposal_id.to_string()).await {
        Ok(logs) => if logs.is_empty() { None } else { Some(logs) },
        Err(_) => None,
    };
    
    // Get basic proposal details
    let proposal = match storage.get_proposal(&proposal_id.to_string()).await {
        Ok(p) => p,
        Err(e) => {
            error!("Error retrieving proposal details: {}", e);
            return Err(internal_error(format!("Error retrieving proposal details: {}", e)).into());
        }
    };
    
    // Return response with execution data
    let response_data = json!({
        "proposal_id": proposal_id.to_string(),
        "title": proposal.title,
        "status": proposal.status,
        "execution_result": execution_result,
        "execution_logs": execution_logs,
        "version": version
    });
    
    Ok(warp::reply::json(&ApiResponse::success("Retrieved execution results", response_data)))
}

/// Handler to list all execution versions for a proposal
async fn list_execution_versions_handler<S>(
    proposal_id: ProposalId,
    storage: Arc<S>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    info!("Listing execution versions for proposal {}", proposal_id);
    
    // Check if proposal exists
    let proposal_exists = storage.get_proposal(&proposal_id.to_string()).await.is_ok();
    
    if !proposal_exists {
        return Err(not_found(format!("Proposal with id {} not found", proposal_id)).into());
    }
    
    // Note: AsyncStorage doesn't yet have a list_execution_versions method
    // For now, we'll return a simple response showing only the latest version
    
    let versions = vec![
        serde_json::json!({
            "version": 1,
            "executed_at": chrono::Utc::now().to_rfc3339(),
            "success": true,
            "summary": "Latest execution result"
        })
    ];
    
    let meta = ResponseMeta {
        total: versions.len() as u64,
        page: 1,
        per_page: versions.len() as u64,
    };
    
    Ok(warp::reply::json(&ApiResponse::success_with_meta(versions, meta)))
} 