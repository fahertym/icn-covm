use serde::{Deserialize, Serialize};
use std::fmt;
use warp::reject::Reject;

/// Standardized API error response format
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub status: u16,
    pub error: String,
    pub message: String,
    pub timestamp: String,
    pub path: String,
}

/// API error types that can be returned by endpoints
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    InternalServerError(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ApiError::NotFound(msg) => format!("Not Found: {}", msg),
            ApiError::BadRequest(msg) => format!("Bad Request: {}", msg),
            ApiError::Unauthorized(msg) => format!("Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => format!("Forbidden: {}", msg),
            ApiError::InternalServerError(msg) => format!("Internal Server Error: {}", msg),
        };
        write!(f, "{}", msg)
    }
}

impl Reject for ApiError {}

/// Helper function to create a Rejection from an ApiError
pub fn reject_with_api_error(error: ApiError) -> warp::Rejection {
    warp::reject::custom(error)
}

/// Create a not found error with the given message
pub fn not_found(msg: &str) -> ApiError {
    ApiError::NotFound(msg.to_string())
}

/// Create a bad request error with the given message
pub fn bad_request(msg: &str) -> ApiError {
    ApiError::BadRequest(msg.to_string())
}

/// Create an unauthorized error with the given message
pub fn unauthorized(msg: &str) -> ApiError {
    ApiError::Unauthorized(msg.to_string())
}

/// Create a forbidden error with the given message
pub fn forbidden(msg: &str) -> ApiError {
    ApiError::Forbidden(msg.to_string())
}

/// Create an internal server error with the given message
pub fn internal_error(msg: &str) -> ApiError {
    ApiError::InternalServerError(msg.to_string())
}

/// Convert any error type to an internal server error
pub fn any_error<E: std::error::Error>(err: E) -> ApiError {
    ApiError::InternalServerError(format!("{}", err))
} 