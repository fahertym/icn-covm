pub mod proposal_api;
pub mod dsl_api;
pub mod v1;
pub mod auth;
pub mod error;
pub mod storage;

use crate::storage::traits::{Storage, StorageExtensions, AsyncStorageExtensions, StorageBackend, JsonStorage};
use crate::vm::VM;
use error::{ApiError, ErrorResponse, reject_with_api_error};
use std::fmt::Debug;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::Method;
use warp::Filter;
use warp::http::header::{HeaderMap, HeaderValue};
use warp::filters::cors::CorsForbidden;
use warp::reject::Reject;
use warp::{Reply, Rejection};

/// Initializes and runs the HTTP API server
pub async fn start_api_server<S>(vm: VM<Arc<Mutex<S>>>, port: u16) -> Result<(), Box<dyn std::error::Error>>
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + Debug + 'static,
{
    // Read environment variables or use defaults
    let allowed_origins = env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "*".to_string());
    
    // Wrap VM in Arc for sharing across routes
    let vm_arc = Arc::new(vm);
    
    // Load legacy routes for backward compatibility
    let legacy_proposal_routes = proposal_api::get_routes(vm_arc.clone());
    let legacy_dsl_routes = dsl_api::get_routes(vm_arc.clone());
    
    // Load v1 API routes (versioned)
    let v1_routes = v1::get_routes((*vm_arc).clone());
    
    // Set up CORS configuration
    let cors = warp::cors()
        .allow_methods(&[Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(vec!["Content-Type", "Authorization", "x-api-key", "x-identity-token"])
        .allow_origins(allowed_origins.split(',').map(|s| s.trim()).collect::<Vec<_>>())
        .allow_credentials(true);
    
    // Combine all routes
    let routes = legacy_proposal_routes
        .or(legacy_dsl_routes)
        .or(v1_routes);
    
    // Apply middleware
    let routes = routes
        .with(cors)
        .with(warp::log("api"))
        .recover(handle_rejection);
    
    // Add security headers
    let routes = routes.map(|reply| {
        security_headers(reply)
    });
    
    println!("Starting API server on port {}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
    
    Ok(())
}

/// Adds security headers to a reply
fn security_headers<T: Reply>(reply: T) -> impl Reply {
    let reply = warp::reply::with_header(
        reply,
        "X-Content-Type-Options", 
        "nosniff"
    );
    let reply = warp::reply::with_header(
        reply,
        "X-Frame-Options", 
        "DENY"
    );
    let reply = warp::reply::with_header(
        reply,
        "X-XSS-Protection", 
        "1; mode=block"
    );
    let reply = warp::reply::with_header(
        reply,
        "Referrer-Policy", 
        "strict-origin-when-cross-origin"
    );
    let reply = warp::reply::with_header(
        reply,
        "Strict-Security-Policy", 
        "default-src 'self'; frame-ancestors 'none'; form-action 'self';"
    );
    reply
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
        path: String::new(), // Default to empty string since path() method is not available
    });

    Ok(warp::reply::with_status(json, code))
}
