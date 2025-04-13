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
use crate::api::v1::models::*;
use crate::api::v1::errors::*;
use crate::governance::proposal_lifecycle::ProposalLifecycle;
use crate::vm::VM;

/// Query parameters for execution result requests
#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionVersionQuery {
    /// Optional version to retrieve (if not specified, latest will be returned)
    pub version: Option<u64>,
}

/// Creates routes for execution result endpoints
pub fn execution_result_routes<S>(
    storage: Arc<S>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: Storage + StorageExtensions + AsyncStorage + Send + Sync + Clone + Debug + 'static,
{
    let base = warp::path("proposals")
        .and(warp::path::param::<String>())
        .and(warp::path("execution"));

    // GET /proposals/:id/execution - Get the execution result
    let get_execution = base
        .and(warp::get())
        .and(warp::path::end())
        .and(warp::query::<ExecutionVersionQuery>())
        .and(with_storage(storage.clone()))
        .and_then(get_execution_results_handler);

    // GET /proposals/:id/execution/versions - List all execution versions
    let list_versions = warp::path("proposals")
        .and(warp::path::param::<String>())
        .and(warp::path("execution"))
        .and(warp::path("versions"))
        .and(warp::path::end())
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and_then(list_execution_versions_handler);

    get_execution.or(list_versions)
}

/// Helper to inject storage dependency
fn with_storage<S>(
    storage: Arc<S>,
) -> impl Filter<Extract = (Arc<S>,), Error = std::convert::Infallible> + Clone
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    warp::any().map(move || storage.clone())
}

/// Get execution results handler
pub async fn get_execution_results_handler<S>(
    proposal_id_str: String,
    query: ExecutionVersionQuery,
    storage: Arc<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Parse proposal ID
    let proposal_id = ProposalId::try_from(proposal_id_str.as_str())
        .map_err(|_| ApiError::InvalidProposalId(proposal_id_str.clone()))?;

    // Ensure the proposal exists
    let proposal_lifecycle_key = format!("governance/proposals/{}/lifecycle", proposal_id);
    match storage.get_json::<ProposalLifecycle>(None, "governance", &proposal_lifecycle_key) {
        Ok(mut lifecycle) => {
            // Try to load execution metadata
            let _ = lifecycle.load_execution_metadata(&*storage);
            
            // Check if we're looking for a specific version
            if let Some(version) = query.version {
                // Look for specific version
                match storage.get_execution_result_version(&proposal_id.to_string(), version) {
                    Ok(result) => {
                        // Get execution logs if available
                        let logs = storage
                            .get_proposal_execution_logs(&proposal_id.to_string())
                            .unwrap_or_else(|_| "".to_string());

                        // Build response with metadata
                        let response = json!({
                            "proposal_id": proposal_id.to_string(),
                            "result": result,
                            "logs": logs,
                            "version": version,
                            "metadata": lifecycle.execution_metadata,
                            "execution_status": lifecycle.execution_status
                        });
                        
                        Ok(warp::reply::json(&response))
                    }
                    Err(StorageError::KeyNotFound(_)) => {
                        Err(ApiError::VersionNotFound(proposal_id.to_string(), version).into())
                    }
                    Err(err) => Err(ApiError::StorageError(err.to_string()).into()),
                }
            } else {
                // Look for latest version
                match storage.get_latest_execution_result(&proposal_id.to_string()) {
                    Ok(result) => {
                        // Get execution logs if available
                        let logs = storage
                            .get_proposal_execution_logs(&proposal_id.to_string())
                            .unwrap_or_else(|_| "".to_string());

                        // Get latest version number
                        let latest_version = lifecycle
                            .execution_metadata
                            .as_ref()
                            .map(|meta| meta.version)
                            .unwrap_or(0);

                        // Build response with metadata
                        let response = json!({
                            "proposal_id": proposal_id.to_string(),
                            "result": result,
                            "logs": logs,
                            "version": latest_version,
                            "metadata": lifecycle.execution_metadata,
                            "execution_status": lifecycle.execution_status
                        });
                        
                        Ok(warp::reply::json(&response))
                    }
                    Err(StorageError::KeyNotFound(_)) => {
                        Err(ApiError::NoExecutionResult(proposal_id.to_string()).into())
                    }
                    Err(err) => Err(ApiError::StorageError(err.to_string()).into()),
                }
            }
        }
        Err(StorageError::KeyNotFound(_)) => {
            Err(ApiError::ProposalNotFound(proposal_id.to_string()).into())
        }
        Err(err) => Err(ApiError::StorageError(err.to_string()).into()),
    }
}

/// List execution versions handler
pub async fn list_execution_versions_handler<S>(
    proposal_id_str: String,
    storage: Arc<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    // Parse proposal ID
    let proposal_id = ProposalId::try_from(proposal_id_str.as_str())
        .map_err(|_| ApiError::InvalidProposalId(proposal_id_str.clone()))?;

    // Ensure the proposal exists
    let proposal_lifecycle_key = format!("governance/proposals/{}/lifecycle", proposal_id);
    match storage.get_json::<ProposalLifecycle>(None, "governance", &proposal_lifecycle_key) {
        Ok(mut lifecycle) => {
            // Try to load execution metadata
            let _ = lifecycle.load_execution_metadata(&*storage);
            
            // Get execution versions
            match storage.list_execution_versions(&proposal_id.to_string()) {
                Ok(versions) => {
                    let response = json!({
                        "proposal_id": proposal_id.to_string(),
                        "versions": versions,
                        "current_metadata": lifecycle.execution_metadata,
                        "execution_status": lifecycle.execution_status
                    });
                    Ok(warp::reply::json(&response))
                }
                Err(err) => Err(ApiError::StorageError(err.to_string()).into()),
            }
        }
        Err(StorageError::KeyNotFound(_)) => {
            Err(ApiError::ProposalNotFound(proposal_id.to_string()).into())
        }
        Err(err) => Err(ApiError::StorageError(err.to_string()).into()),
    }
} 