use serde::{Deserialize, Serialize};
use actix_web::{HttpResponse, http::StatusCode};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> 
where 
    T: Serialize,
{
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_pages: Option<u32>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn new(status_code: StatusCode, message: &str, data: Option<T>) -> Self {
        Self {
            status: status_code.as_u16().to_string(),
            message: message.to_string(),
            data,
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: ResponseMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn success(message: &str, data: T) -> Self {
        Self::new(StatusCode::OK, message, Some(data))
    }

    pub fn error(status_code: StatusCode, message: &str) -> Self {
        Self::new(status_code, message, None)
    }

    pub fn created(message: &str, data: T) -> Self {
        Self::new(StatusCode::CREATED, message, Some(data))
    }

    pub fn no_content() -> HttpResponse {
        HttpResponse::NoContent().finish()
    }

    pub fn to_http_response(&self, status_code: StatusCode) -> HttpResponse {
        HttpResponse::build(status_code).json(self)
    }
}

// Helper functions for common response patterns
pub fn ok<T: Serialize>(message: &str, data: T) -> HttpResponse {
    ApiResponse::success(message, data).to_http_response(StatusCode::OK)
}

pub fn created<T: Serialize>(message: &str, data: T) -> HttpResponse {
    ApiResponse::created(message, data).to_http_response(StatusCode::CREATED)
}

pub fn no_content() -> HttpResponse {
    ApiResponse::<()>::no_content()
}

pub fn paginated<T: Serialize>(
    message: &str, 
    data: Vec<T>, 
    page: u32, 
    per_page: u32, 
    total: u64
) -> HttpResponse {
    let total_pages = (total as f64 / per_page as f64).ceil() as u32;
    
    let meta = ResponseMeta {
        page: Some(page),
        per_page: Some(per_page),
        total: Some(total),
        total_pages: Some(total_pages),
    };
    
    ApiResponse::success(message, data)
        .with_meta(meta)
        .to_http_response(StatusCode::OK)
} 