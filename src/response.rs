use serde::{Deserialize, Serialize};
use warp::http::StatusCode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseMeta {
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponse<T> {
    pub status: String,
    pub message: String,
    pub data: T,
    pub meta: Option<ResponseMeta>,
}

impl<T> ApiResponse<T> {
    pub fn new(status: String, message: String, data: T, meta: Option<ResponseMeta>) -> Self {
        Self {
            status,
            message,
            data,
            meta,
        }
    }

    pub fn success(message: &str, data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: message.to_string(),
            data,
            meta: None,
        }
    }

    pub fn success_with_meta(data: T, meta: ResponseMeta) -> Self {
        Self {
            status: "success".to_string(),
            message: "Success".to_string(),
            data,
            meta: Some(meta),
        }
    }

    pub fn error(message: &str, data: T) -> Self {
        Self {
            status: "error".to_string(),
            message: message.to_string(),
            data,
            meta: None,
        }
    }

    pub fn created(message: &str, data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: message.to_string(),
            data,
            meta: None,
        }
    }

    pub fn no_content(data: T) -> Self {
        Self {
            status: "success".to_string(),
            message: "No content".to_string(),
            data,
            meta: None,
        }
    }
}

// Helper functions for common response patterns
pub fn ok<T: Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse::success("Success", data)
}

pub fn created<T: Serialize>(data: T) -> ApiResponse<T> {
    ApiResponse::created("Created successfully", data)
}

pub fn paginated<T: Serialize>(data: T, page: u64, per_page: u64, total: u64) -> ApiResponse<T> {
    let meta = ResponseMeta {
        total,
        page,
        per_page,
    };
    ApiResponse::success_with_meta(data, meta)
} 