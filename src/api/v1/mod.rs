pub mod dsl;
pub mod governance;
pub mod models;
pub mod proposals;

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
    
    // Create Arc-wrapped storage for sharing between routes
    let storage = Arc::new(vm.storage().clone());
    
    // Register v1 routes
    let dsl_routes = base.and(dsl::routes(vm.clone()));
    let governance_routes = base.and(governance::routes(vm.clone()));
    let proposals_routes = base.and(proposals::get_routes(storage, vm));
    
    // Combine all v1 routes
    dsl_routes
        .or(governance_routes)
        .or(proposals_routes)
} 