pub mod proposal_api;
pub mod dsl_api;

use crate::storage::traits::{Storage, StorageExtensions};
use crate::vm::VM;
use std::fmt::Debug;

/// Initializes and runs the HTTP API server
pub async fn start_api_server<S>(vm: VM<S>, port: u16) -> Result<(), Box<dyn std::error::Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Initialize the API with both proposal and DSL endpoints
    let api_routes = proposal_api::get_routes(vm.clone());
    let dsl_routes = dsl_api::get_routes(vm);

    // Combine all routes and start the server
    let routes = api_routes
        .or(dsl_routes)
        .with(warp::cors().allow_any_origin())
        .recover(handle_rejection);

    println!("Starting API server on port {}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;

    Ok(())
}

/// Common error handler for API rejections
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    let message = if err.is_not_found() {
        "Not Found".to_string()
    } else {
        format!("Internal Server Error: {:?}", err)
    };

    let json = warp::reply::json(&proposal_api::ErrorResponse { message });
    Ok(warp::reply::with_status(json, warp::http::StatusCode::BAD_REQUEST))
}
