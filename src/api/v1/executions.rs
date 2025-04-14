use warp::{Filter, Rejection, Reply};
use std::fmt::Debug;
use std::sync::Arc;

use crate::api::error::{ApiError, reject_with_api_error};
use crate::storage::traits::{Storage, StorageExtensions};
use crate::storage::errors::StorageError;
use crate::vm::VM;
use crate::response::{ApiResponse, ResponseMeta};

/// Query parameters for execution result endpoints
#[derive(Debug, Default, serde::Deserialize)]
pub struct ExecutionResultQuery {
    /// Optional version to retrieve (if not provided, the latest version is returned)
    pub version: Option<u32>,
}

/// Routes for handling execution results
pub fn execution_result_routes<S>(
    vm: VM<S>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let storage = Arc::new(vm.storage_backend.as_ref().unwrap().clone());

    // GET /proposals/:id/execution - Get latest execution result or specific version based on query params
    let get_execution_result = warp::path!("proposals" / String / "execution")
        .and(warp::get())
        .and(warp::query::<ExecutionResultQuery>())
        .and(with_storage(storage.clone()))
        .and_then(get_execution_result_handler);

    // GET /proposals/:id/execution/:version - Get specific version of execution result
    let get_execution_result_version = warp::path!("proposals" / String / "execution" / u32)
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and_then(get_execution_result_version_handler);

    // GET /proposals/:id/execution/versions - List all execution result versions for a proposal
    let list_execution_versions = warp::path!("proposals" / String / "execution" / "versions")
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and_then(list_execution_versions_handler);

    // Combine all routes
    get_execution_result
        .or(get_execution_result_version)
        .or(list_execution_versions)
}

/// Filter to add storage to route handlers
fn with_storage<S>(
    storage: Arc<S>,
) -> impl Filter<Extract = (Arc<S>,), Error = std::convert::Infallible> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    warp::any().map(move || storage.clone())
}

/// Handler for getting the execution result (latest or specific version)
async fn get_execution_result_handler<S>(
    proposal_id: String,
    query: ExecutionResultQuery,
    storage: Arc<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    if let Some(version) = query.version {
        // Get specific version
        match storage.get_proposal_execution_result_versioned(&proposal_id, version).await {
            Ok(result) => Ok(warp::reply::json(&ApiResponse::success("Retrieved execution result", result))),
            Err(StorageError::NotFound) => Err(reject_with_api_error(
                ApiError::NotFound(format!("Execution result for proposal {} version {} not found", proposal_id, version))
            )),
            Err(err) => Err(reject_with_api_error(
                ApiError::InternalServerError(format!("Failed to retrieve execution result: {}", err))
            )),
        }
    } else {
        // Get latest version
        match storage.get_latest_proposal_execution_result(&proposal_id).await {
            Ok(result) => Ok(warp::reply::json(&ApiResponse::success("Retrieved latest execution result", result))),
            Err(StorageError::NotFound) => Err(reject_with_api_error(
                ApiError::NotFound(format!("No execution results found for proposal {}", proposal_id))
            )),
            Err(err) => Err(reject_with_api_error(
                ApiError::InternalServerError(format!("Failed to retrieve execution result: {}", err))
            )),
        }
    }
}

/// Handler for getting a specific version of execution result
async fn get_execution_result_version_handler<S>(
    proposal_id: String,
    version: u32,
    storage: Arc<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    match storage.get_proposal_execution_result_versioned(&proposal_id, version).await {
        Ok(result) => Ok(warp::reply::json(&ApiResponse::success("Retrieved execution result version", result))),
        Err(StorageError::NotFound) => Err(reject_with_api_error(
            ApiError::NotFound(format!("Execution result for proposal {} version {} not found", proposal_id, version))
        )),
        Err(err) => Err(reject_with_api_error(
            ApiError::InternalServerError(format!("Failed to retrieve execution result: {}", err))
        )),
    }
}

/// Handler for listing all execution result versions for a proposal
async fn list_execution_versions_handler<S>(
    proposal_id: String,
    storage: Arc<S>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    match storage.list_proposal_execution_versions(&proposal_id).await {
        Ok(versions) => Ok(warp::reply::json(&ApiResponse::success("Retrieved execution versions", versions))),
        Err(StorageError::NotFound) => Err(reject_with_api_error(
            ApiError::NotFound(format!("No execution versions found for proposal {}", proposal_id))
        )),
        Err(err) => Err(reject_with_api_error(
            ApiError::InternalServerError(format!("Failed to retrieve execution versions: {}", err))
        )),
    }
} 