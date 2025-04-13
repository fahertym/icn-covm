use std::convert::Infallible;
use std::fmt::Debug;
use warp::{self, Filter, Rejection, Reply};

use crate::storage::traits::{ExecutionVersionMeta, RetryHistoryRecord, Storage, StorageExtensions, StorageResult};
use crate::vm::VM;
use crate::auth::AuthContext;
use crate::response::{
    ApiResponse, success_response, error_response, paginated_response,
};

/// Query parameters for the execution result endpoints
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExecutionResultQuery {
    /// The specific version to retrieve
    pub version: Option<u64>,
    /// Whether to include logs in the response
    pub include_logs: Option<bool>,
}

/// Creates all the routes for execution results
pub fn execution_result_routes<S>(
    vm: VM<S>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let vm_filter = warp::any().map(move || vm.clone());

    // Route: GET /proposals/:id/execution
    // Gets the latest execution result or a specific version if query param is provided
    let get_execution_result = warp::path!("proposals" / String / "execution")
        .and(warp::get())
        .and(warp::query::<ExecutionResultQuery>())
        .and(vm_filter.clone())
        .and_then(get_execution_result_handler);

    // Route: GET /proposals/:id/execution/:version
    // Gets a specific version of an execution result
    let get_execution_version = warp::path!("proposals" / String / "execution" / u64)
        .and(warp::get())
        .and(vm_filter.clone())
        .and_then(get_execution_result_version_handler);

    // Route: GET /proposals/:id/execution/versions
    // Lists all execution result versions for a proposal
    let list_execution_versions = warp::path!("proposals" / String / "execution" / "versions")
        .and(warp::get())
        .and(vm_filter.clone())
        .and_then(list_execution_versions_handler);

    // Route: GET /proposals/:id/execution/retry-history
    // Gets the retry history for a proposal
    let get_retry_history = warp::path!("proposals" / String / "execution" / "retry-history")
        .and(warp::get())
        .and(vm_filter.clone())
        .and_then(get_retry_history_handler);

    // Combine all routes
    get_execution_result
        .or(get_execution_version)
        .or(list_execution_versions)
        .or(get_retry_history)
}

/// Handler for getting the latest execution result or a specific version
async fn get_execution_result_handler<S>(
    proposal_id: String,
    query: ExecutionResultQuery,
    vm: VM<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm.storage_backend.as_ref().ok_or_else(|| {
        warp::reject::custom(ExecutionResultError::StorageError(
            "Storage backend not configured".to_string(),
        ))
    })?;

    let result = if let Some(version) = query.version {
        // Get a specific version
        match storage.get_proposal_execution_result_versioned(&proposal_id, version) {
            Ok(execution_result) => {
                let include_logs = query.include_logs.unwrap_or(false);
                let mut response = success_response(execution_result);
                
                // Include logs if requested
                if include_logs {
                    match storage.get_proposal_execution_logs(&proposal_id) {
                        Ok(logs) => {
                            response.metadata = Some(serde_json::json!({
                                "logs": logs,
                                "version": version,
                            }));
                        }
                        Err(e) => {
                            return Ok(error_response(
                                format!("Failed to get execution logs: {}", e),
                                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                            ));
                        }
                    }
                }
                
                response
            }
            Err(e) => {
                return Ok(error_response(
                    format!("Failed to get execution result version {}: {}", version, e),
                    warp::http::StatusCode::NOT_FOUND,
                ));
            }
        }
    } else {
        // Get the latest version
        match storage.get_latest_execution_result(&proposal_id) {
            Ok(execution_result) => {
                // Include logs if requested
                let include_logs = query.include_logs.unwrap_or(false);
                let mut response = success_response(execution_result);
                
                if include_logs {
                    match storage.get_proposal_execution_logs(&proposal_id) {
                        Ok(logs) => {
                            // Try to get the latest version for metadata
                            let version = match storage.get_latest_execution_result_version(&proposal_id) {
                                Ok(v) => v,
                                Err(_) => 0,
                            };
                            
                            response.metadata = Some(serde_json::json!({
                                "logs": logs,
                                "version": version,
                            }));
                        }
                        Err(e) => {
                            return Ok(error_response(
                                format!("Failed to get execution logs: {}", e),
                                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                            ));
                        }
                    }
                }
                
                response
            }
            Err(e) => {
                return Ok(error_response(
                    format!("Failed to get latest execution result: {}", e),
                    warp::http::StatusCode::NOT_FOUND,
                ));
            }
        }
    };

    Ok(warp::reply::json(&result))
}

/// Handler for getting a specific version of an execution result
async fn get_execution_result_version_handler<S>(
    proposal_id: String,
    version: u64,
    vm: VM<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm.storage_backend.as_ref().ok_or_else(|| {
        warp::reject::custom(ExecutionResultError::StorageError(
            "Storage backend not configured".to_string(),
        ))
    })?;

    match storage.get_proposal_execution_result_versioned(&proposal_id, version) {
        Ok(result) => {
            let response = success_response(result);
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = error_response(
                format!("Failed to get execution result version {}: {}", version, e),
                warp::http::StatusCode::NOT_FOUND,
            );
            Ok(warp::reply::json(&response))
        }
    }
}

/// Handler for listing all execution versions for a proposal
async fn list_execution_versions_handler<S>(
    proposal_id: String,
    vm: VM<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm.storage_backend.as_ref().ok_or_else(|| {
        warp::reject::custom(ExecutionResultError::StorageError(
            "Storage backend not configured".to_string(),
        ))
    })?;

    match storage.list_execution_versions(&proposal_id) {
        Ok(versions) => {
            // Convert to a paginated response for consistency
            let response = paginated_response(versions, versions.len(), 0, versions.len());
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = error_response(
                format!("Failed to list execution versions: {}", e),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            );
            Ok(warp::reply::json(&response))
        }
    }
}

/// Handler for getting the retry history for a proposal
async fn get_retry_history_handler<S>(
    proposal_id: String,
    vm: VM<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm.storage_backend.as_ref().ok_or_else(|| {
        warp::reject::custom(ExecutionResultError::StorageError(
            "Storage backend not configured".to_string(),
        ))
    })?;

    match storage.get_proposal_retry_history(&proposal_id) {
        Ok(history) => {
            // Add cooldown information if available and if the last attempt failed
            let mut response_data = history;
            
            // Try to get the cooldown info if there is at least one record
            if !response_data.is_empty() && response_data[0].status == "failed" {
                if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(&response_data[0].timestamp) {
                    use crate::utils::time;
                    use crate::governance::proposal_lifecycle::COOLDOWN_DURATION;
                    
                    let utc_timestamp = timestamp.with_timezone(&chrono::Utc);
                    let remaining = time::get_cooldown_remaining(&utc_timestamp.to_string(), COOLDOWN_DURATION);
                    
                    let cooldown_info = if remaining.num_seconds() > 0 {
                        serde_json::json!({
                            "cooldown_active": true,
                            "remaining_seconds": remaining.num_seconds(),
                            "formatted_remaining": time::format_duration(remaining),
                        })
                    } else {
                        serde_json::json!({
                            "cooldown_active": false,
                            "remaining_seconds": 0,
                            "formatted_remaining": "Ready for retry",
                        })
                    };
                    
                    let mut response = success_response(response_data);
                    response.metadata = Some(cooldown_info);
                    return Ok(warp::reply::json(&response));
                }
            }
            
            // If no cooldown info, just return the history
            let response = success_response(response_data);
            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            let response = error_response(
                format!("Failed to get retry history: {}", e),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            );
            Ok(warp::reply::json(&response))
        }
    }
}

/// Custom error type for execution result operations
#[derive(Debug)]
enum ExecutionResultError {
    StorageError(String),
}

impl warp::reject::Reject for ExecutionResultError {} 