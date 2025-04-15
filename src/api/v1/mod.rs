pub mod dsl;
pub mod executions;
pub mod models;
pub mod proposals;

// Use the handlers submodule
pub mod handlers;

use crate::storage::traits::{Storage, StorageExtensions, AsyncStorageExtensions, StorageBackend, JsonStorage};
use crate::vm::VM;
use warp::{Filter, Rejection, Reply};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Returns all v1 API routes
pub fn get_routes<S>(vm: VM<Arc<Mutex<S>>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    // Base path for v1 API
    let base = warp::path("api").and(warp::path("v1"));
    
    // Create wrapped VM and storage for sharing
    let vm_arc = Arc::new(vm.clone());
    let storage_arc = vm.storage_backend.as_ref().unwrap().clone();
    
    // Register v1 routes
    let dsl_routes = base.and(dsl::get_routes(storage_arc.clone(), vm_arc.clone()));
    let proposals_routes = base.and(proposals::get_routes(storage_arc, vm_arc));
    let execution_result_routes = base.and(executions::execution_result_routes(vm));
    
    // Combine all v1 routes
    dsl_routes
        .or(proposals_routes)
        .or(execution_result_routes)
}

// Helper function to convert a concrete storage implementation to async storage
pub fn with_storage<S>(storage: Arc<Mutex<S>>) -> impl Filter<Extract = (Arc<Arc<Mutex<S>>>,), Error = std::convert::Infallible> + Clone
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + Send + Sync + 'static
{
    let async_storage = Arc::new(storage);
    warp::any().map(move || async_storage.clone())
} 