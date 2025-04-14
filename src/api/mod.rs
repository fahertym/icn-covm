pub mod proposal_api;
pub mod dsl_api;
pub mod v1;
pub mod auth;
pub mod error;
pub mod storage;

use crate::storage::traits::{Storage, StorageExtensions, AsyncStorageExtensions, StorageBackend};
use crate::vm::VM;
use error::{ApiError, ErrorResponse, reject_with_api_error};
use std::fmt::Debug;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::Method;
use warp::filters::cors::CorsForbidden;
use warp::reject::Reject;

/// Initializes and runs the HTTP API server
pub async fn start_api_server<S>(vm: VM<Arc<Mutex<S>>>, port: u16) -> Result<(), Box<dyn std::error::Error>>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Read environment variables or use defaults
    let allowed_origins = env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "*".to_string());
    
    // Load legacy routes for backward compatibility
    let legacy_proposal_routes = proposal_api::get_routes(vm.clone());
    let legacy_dsl_routes = dsl_api::get_routes(vm.clone());
    
    // Load v1 API routes (versioned)
    let v1_routes = v1::get_routes(vm.clone());
    
    // Set up CORS configuration
    let cors = warp::cors()
        .allow_methods(&[
            Method::GET, 
            Method::POST, 
            Method::PUT, 
            Method::DELETE, 
            Method::OPTIONS,
        ])
        .allow_headers(vec!["Content-Type", "Authorization", "X-API-Key"])
        .allow_origin(allowed_origins.split(',').collect::<Vec<_>>().as_slice())
        .max_age(3600)
        .build();
    
    // Combine all routes with proper security headers and error handling
    let routes = legacy_proposal_routes
        .or(legacy_dsl_routes)
        .or(v1_routes)
        .with(cors)
        .with(security_headers())
        .with(warp::log("api"))
        .recover(handle_rejection);
    
    println!("Starting API server on port {}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
    
    Ok(())
}

/// Adds security headers to all responses
fn security_headers() -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::reply::with::header("X-Content-Type-Options", "nosniff")
        .and(warp::reply::with::header("X-Frame-Options", "DENY"))
        .and(warp::reply::with::header("X-XSS-Protection", "1; mode=block"))
        .and(warp::reply::with::header("Referrer-Policy", "strict-origin-when-cross-origin"))
        .and(warp::reply::with::header("Cache-Control", "no-store"))
        .and(warp::reply::with::header("Pragma", "no-cache"))
        .map(|reply| reply)
}

/// Common error handler for API rejections
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    let (code, message, error_type) = if err.is_not_found() {
        (
            warp::http::StatusCode::NOT_FOUND,
            "Not Found".to_string(),
            "not_found"
        )
    } else if let Some(e) = err.find::<ApiError>() {
        match e {
            ApiError::NotFound(msg) => (
                warp::http::StatusCode::NOT_FOUND,
                msg.clone(),
                "not_found"
            ),
            ApiError::BadRequest(msg) => (
                warp::http::StatusCode::BAD_REQUEST,
                msg.clone(),
                "bad_request"
            ),
            ApiError::Unauthorized(msg) => (
                warp::http::StatusCode::UNAUTHORIZED,
                msg.clone(),
                "unauthorized"
            ),
            ApiError::Forbidden(msg) => (
                warp::http::StatusCode::FORBIDDEN,
                msg.clone(),
                "forbidden"
            ),
            ApiError::InternalServerError(msg) => (
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "server_error"
            ),
        }
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        (
            warp::http::StatusCode::BAD_REQUEST,
            format!("Invalid request body: {}", e),
            "invalid_body"
        )
    } else if let Some(e) = err.find::<CorsForbidden>() {
        (
            warp::http::StatusCode::FORBIDDEN,
            format!("CORS forbidden: {}", e),
            "cors_error"
        )
    } else {
        (
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled error: {:?}", err),
            "server_error"
        )
    };

    // Create a standardized error response
    let json = warp::reply::json(&ErrorResponse {
        status: code.as_u16(),
        error: error_type.into(),
        message,
        timestamp: chrono::Utc::now().to_rfc3339(),
        path: err.path().map(|p| p.to_string()).unwrap_or_default(),
    });

    Ok(warp::reply::with_status(json, code))
}
