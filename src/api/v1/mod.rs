pub mod dsl;
pub mod executions;
pub mod models;
pub mod proposals;

// Use the handlers submodule, not the file
pub mod handlers {
    pub use super::handlers::execution::*;
}

use crate::storage::traits::{Storage, StorageExtensions};
use crate::vm::VM;
use warp::{Filter, Rejection, Reply};
use std::fmt::Debug;
use std::sync::Arc;

/// Returns all v1 API routes
pub fn get_routes<S>(vm: VM<S>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Base path for v1 API
    let base = warp::path("api").and(warp::path("v1"));
    
    // Create wrapped VM and storage for sharing
    let vm_arc = Arc::new(vm.clone());
    let storage_arc = Arc::new(vm.storage_backend.as_ref().unwrap().clone());
    
    // Register v1 routes
    let dsl_routes = base.and(dsl::get_routes(storage_arc.clone(), vm_arc.clone()));
    let proposals_routes = base.and(proposals::get_routes(storage_arc, vm_arc));
    let execution_result_routes = base.and(executions::execution_result_routes(vm));
    
    // Combine all v1 routes
    dsl_routes
        .or(proposals_routes)
        .or(execution_result_routes)
} 