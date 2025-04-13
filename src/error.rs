use std::fmt;
use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use validator::ValidationErrors;
use crate::response::ApiResponse;

#[derive(Debug)]
pub enum AppError {
    // Database errors
    DatabaseError(String),
    EntityNotFound(String),
    
    // Authentication errors
    AuthenticationError(String),
    UnauthorizedError(String),
    
    // Validation errors
    ValidationError(ValidationErrors),
    
    // External service errors
    ExternalServiceError(String),
    
    // General errors
    BadRequest(String),
    InternalServerError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::EntityNotFound(msg) => write!(f, "Not found: {}", msg),
            Self::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            Self::UnauthorizedError(msg) => write!(f, "Unauthorized: {}", msg),
            Self::ValidationError(err) => write!(f, "Validation error: {:?}", err),
            Self::ExternalServiceError(msg) => write!(f, "External service error: {}", msg),
            Self::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            Self::InternalServerError(msg) => write!(f, "Internal server error: {}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::EntityNotFound(_) => StatusCode::NOT_FOUND,
            Self::AuthenticationError(_) => StatusCode::UNAUTHORIZED,
            Self::UnauthorizedError(_) => StatusCode::FORBIDDEN,
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let error_message = self.to_string();
        
        let errors = match self {
            Self::ValidationError(validation_errors) => {
                let validation_errors_map = validation_errors
                    .field_errors()
                    .iter()
                    .map(|(field, errors)| {
                        let error_messages: Vec<String> = errors
                            .iter()
                            .map(|error| error.message.clone().unwrap_or_else(|| "Invalid input".into()).to_string())
                            .collect();
                        (field.to_string(), serde_json::json!(error_messages))
                    })
                    .collect::<serde_json::Map<String, serde_json::Value>>();
                
                Some(serde_json::json!(validation_errors_map))
            },
            _ => None,
        };
        
        let mut api_response = ApiResponse::<()>::error(status, &error_message);
        
        if let Some(validation_errors) = errors {
            let meta = serde_json::json!({
                "errors": validation_errors
            });
            
            api_response = api_response.with_custom_meta(meta);
        }
        
        api_response.to_http_response(status)
    }
}

// Conversion from other error types
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::EntityNotFound("Requested resource not found".into()),
            _ => Self::DatabaseError(err.to_string()),
        }
    }
}

impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        Self::ValidationError(errors)
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::AuthenticationError(format!("JWT error: {}", err))
    }
}

// For convenient error conversion in application code
pub type AppResult<T> = Result<T, AppError>; 