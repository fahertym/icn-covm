use warp::{Filter, Rejection, Reply};
use std::sync::Arc;
use std::fmt::Debug;
use crate::storage::traits::{Storage, StorageExtensions};
use crate::api::auth::{with_auth, AuthInfo};
use crate::api::error::{not_found, internal_error};
use crate::api::storage::AsyncStorage;
use crate::models::ProposalId;
use crate::response::ApiResponse;
use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Query parameters for execution result requests
#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionVersionQuery {
    /// Optional version parameter
    pub version: Option<u64>,
}

/// Creates routes for execution result endpoints
pub fn execution_result_routes<S>(
    storage: Arc<S>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: Storage + StorageExtensions + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    let storage_filter = warp::any().map(move || storage.clone());
    
    // GET /proposals/:id/execution - Get execution result (latest or specific version)
    let get_execution_result = warp::path!("proposals" / ProposalId / "execution")
        .and(warp::get())
        .and(warp::query::<ExecutionVersionQuery>())
        .and(storage_filter.clone())
        .and(with_auth())
        .and_then(get_execution_results_handler);
    
    // GET /proposals/:id/execution/versions - List all execution versions
    let list_execution_versions = warp::path!("proposals" / ProposalId / "execution" / "versions")
        .and(warp::get())
        .and(storage_filter.clone())
        .and(with_auth())
        .and_then(list_execution_versions_handler);
    
    get_execution_result.or(list_execution_versions)
}

/// Handler for retrieving execution results for a proposal
pub async fn get_execution_results_handler(
    proposal_id: ProposalId,
    query: ExecutionVersionQuery,
    storage: Arc<impl Storage + StorageExtensions + AsyncStorage + Send + Sync>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    info!("Getting execution results for proposal {}", proposal_id);
    
    // Check if proposal exists
    let proposal_exists = async {
        // Check if the key exists using contains
        let namespace = "governance";
        let key = format!("proposals/{}", proposal_id);
        storage.contains(None, namespace, &key)
    }.await.map_err(|e| {
        error!("Error checking if proposal exists: {}", proposal_id);
        internal_error(format!("Error checking proposal: {}", e))
    })?;
    
    if !proposal_exists {
        return Err(not_found(format!("Proposal with id {} not found", proposal_id)).into());
    }
    
    // Get execution result based on version
    let execution_result = if let Some(version) = query.version {
        // Get specific version
        async {
            storage.get_proposal_execution_result_versioned(&proposal_id.to_string(), version)
        }.await.map_or_else(
            |e| {
                warn!("No execution result found for proposal {} version {}: {}", proposal_id, version, e);
                serde_json::Value::Null
            },
            |result| {
                // Try to parse the result as JSON for better presentation
                serde_json::from_str(&result).unwrap_or_else(|_| serde_json::Value::String(result))
            }
        )
    } else {
        // Get latest version
        async {
            storage.get_latest_execution_result(&proposal_id.to_string())
        }.await.map_or_else(
            |e| {
                warn!("No execution result found for proposal {}: {}", proposal_id, e);
                serde_json::Value::Null
            },
            |result| {
                // Try to parse the result as JSON for better presentation
                serde_json::from_str(&result).unwrap_or_else(|_| serde_json::Value::String(result))
            }
        )
    };
    
    // Get execution logs if available
    let execution_logs = async {
        storage.get_proposal_execution_logs(&proposal_id.to_string())
    }.await.map_or_else(
        |_| None,
        |logs| if logs.is_empty() { None } else { Some(logs) }
    );
    
    // Get basic proposal details
    let proposal = storage.get_proposal(&proposal_id.to_string()).await.map_err(|e| {
        error!("Error retrieving proposal details: {}", e);
        internal_error(format!("Error retrieving proposal details: {}", e))
    })?;
    
    // Get version metadata if applicable
    let version_info = if let Some(version) = query.version {
        // Try to get metadata for this specific version
        let all_versions = async {
            storage.list_execution_versions(&proposal_id.to_string())
        }.await.unwrap_or_default();
        
        all_versions.into_iter()
            .find(|v| v.version == version)
            .map(|v| json!({
                "version": v.version,
                "executed_at": v.executed_at,
                "success": v.success,
                "summary": v.summary
            }))
    } else {
        // Get latest version info
        let latest_version = async {
            storage.get_latest_execution_result_version(&proposal_id.to_string())
        }.await.ok();
        
        if let Some(ver) = latest_version {
            let all_versions = async {
                storage.list_execution_versions(&proposal_id.to_string())
            }.await.unwrap_or_default();
            
            all_versions.into_iter()
                .find(|v| v.version == ver)
                .map(|v| json!({
                    "version": v.version,
                    "executed_at": v.executed_at,
                    "success": v.success,
                    "summary": v.summary
                }))
        } else {
            None
        }
    };
    
    // Return response with execution data
    let mut response_data = json!({
        "proposal_id": proposal_id.to_string(),
        "title": proposal.title,
        "status": proposal.status,
        "execution_result": execution_result,
        "execution_logs": execution_logs
    });
    
    // Add version info if available
    if let Some(version_meta) = version_info {
        response_data.as_object_mut().unwrap().insert("version_info".to_string(), version_meta);
    }
    
    Ok(warp::reply::json(&ApiResponse::success("Retrieved execution results", response_data)))
}

/// Handler for listing all execution versions for a proposal
pub async fn list_execution_versions_handler(
    proposal_id: ProposalId,
    storage: Arc<impl Storage + StorageExtensions + AsyncStorage + Send + Sync>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    info!("Listing execution versions for proposal {}", proposal_id);
    
    // Check if proposal exists
    let proposal_exists = async {
        // Check if the key exists using contains
        let namespace = "governance";
        let key = format!("proposals/{}", proposal_id);
        storage.contains(None, namespace, &key)
    }.await.map_err(|e| {
        error!("Error checking if proposal exists: {}", proposal_id);
        internal_error(format!("Error checking proposal: {}", e))
    })?;
    
    if !proposal_exists {
        return Err(not_found(format!("Proposal with id {} not found", proposal_id)).into());
    }
    
    // Get all versions
    let versions = async {
        storage.list_execution_versions(&proposal_id.to_string())
    }.await.map_err(|e| {
        warn!("Error retrieving execution versions: {}", e);
        internal_error(format!("Error retrieving execution versions: {}", e))
    })?;
    
    // Convert storage model to API model
    let api_versions: Vec<crate::api::v1::models::ExecutionVersionMeta> = versions.into_iter()
        .map(|v| crate::api::v1::models::ExecutionVersionMeta {
            version: v.version,
            executed_at: v.executed_at,
            success: v.success,
            summary: v.summary,
        })
        .collect();
    
    let response = crate::api::v1::models::ExecutionVersionsResponse {
        total: api_versions.len(),
        versions: api_versions,
    };
    
    Ok(warp::reply::json(&ApiResponse::success("Retrieved execution versions", response)))
} 