use serde::{Deserialize, Serialize};
use thiserror::Error;
use warp::{
    filters::{body::BodyDeserializeError, cors::CorsForbidden},
    http::StatusCode,
    reject::Reject,
    Rejection, Reply,
};

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Not Found: {0}")]
    NotFound(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
    
    #[error("Bad Request: {0}")]
    BadRequest(String),
    
    #[error("Invalid Input: {0}")]
    InvalidInput(String),
    
    #[error("Invalid Operation: {0}")]
    InvalidOperation(String),
    
    #[error("Internal Error: {0}")]
    InternalError(String),
}

impl Reject for ApiError {}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
    pub status: String,
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let code;
    let message;
    let status;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not Found".to_string();
        status = "error".to_string();
    } else if let Some(api_error) = err.find::<ApiError>() {
        match api_error {
            ApiError::NotFound(msg) => {
                code = StatusCode::NOT_FOUND;
                message = msg.clone();
                status = "error".to_string();
            }
            ApiError::Unauthorized(msg) => {
                code = StatusCode::UNAUTHORIZED;
                message = msg.clone();
                status = "error".to_string();
            }
            ApiError::Forbidden(msg) => {
                code = StatusCode::FORBIDDEN;
                message = msg.clone();
                status = "error".to_string();
            }
            ApiError::BadRequest(msg) | ApiError::InvalidInput(msg) => {
                code = StatusCode::BAD_REQUEST;
                message = msg.clone();
                status = "error".to_string();
            }
            ApiError::InvalidOperation(msg) => {
                code = StatusCode::UNPROCESSABLE_ENTITY;
                message = msg.clone();
                status = "error".to_string();
            }
            ApiError::InternalError(msg) => {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = msg.clone();
                status = "error".to_string();
            }
        }
    } else if let Some(e) = err.find::<CorsForbidden>() {
        code = StatusCode::FORBIDDEN;
        message = format!("CORS forbidden: {}", e);
        status = "error".to_string();
    } else if let Some(e) = err.find::<BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        message = format!("Invalid request data: {}", e);
        status = "error".to_string();
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error".to_string();
        status = "error".to_string();
    }

    let json = warp::reply::json(&ErrorResponse {
        code: code.as_u16(),
        message,
        status,
    });

    Ok(warp::reply::with_status(json, code))
} 