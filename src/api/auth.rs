use crate::api::error::{ApiError, reject_with_api_error};
use crate::storage::traits::{Storage, StorageExtensions};
use crate::vm::VM;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

/// Authentication result containing the authenticated user ID
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub user_id: String,
    pub did: Option<String>,
    pub roles: Vec<String>,
}

/// Authentication filter for secure endpoints 
pub fn with_auth<S>() -> impl Filter<Extract = (AuthInfo,), Error = warp::Rejection> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    warp::header::<String>("authorization")
        .and_then(|token: String| async move {
            // Extract the token type (Bearer, Basic, etc.)
            let parts: Vec<&str> = token.split_whitespace().collect();
            
            if parts.len() != 2 || parts[0].to_lowercase() != "bearer" {
                return Err(reject_with_api_error(
                    ApiError::Unauthorized("Invalid authorization header format".to_string())
                ));
            }
            
            let token = parts[1];
            
            // This is a placeholder that would validate a token against a proper auth system
            // For now it just creates a mock auth info for development
            if token.is_empty() {
                return Err(reject_with_api_error(
                    ApiError::Unauthorized("Invalid token".to_string())
                ));
            }
            
            // In a real implementation, verify the token and extract user info
            Ok(AuthInfo {
                user_id: "user123".to_string(), // Replace with actual user ID from token
                did: Some("did:example:123456789abcdefghi".to_string()), // Replace with actual DID
                roles: vec!["user".to_string()], // Replace with actual roles
            })
        })
        .or(warp::any().map(|| {
            // For development, return a mock anonymous user
            // This would be removed in production
            AuthInfo {
                user_id: "anonymous".to_string(),
                did: None,
                roles: vec!["anonymous".to_string()],
            }
        }))
}

/// Middleware to check if user has a specific role
pub fn require_role(
    auth_info: AuthInfo,
    role: &str,
) -> Result<AuthInfo, warp::Rejection> {
    if auth_info.roles.contains(&role.to_string()) {
        Ok(auth_info)
    } else {
        Err(reject_with_api_error(
            ApiError::Forbidden("Insufficient permissions".to_string())
        ))
    }
}

/// Validate a DID proof
pub async fn validate_did_proof(
    proof: &str,
    did: &str,
) -> Result<bool, warp::Rejection> {
    // This is a placeholder for DID verification logic
    // In a real implementation, this would verify the proof against the DID
    
    Ok(true) // Mock validation result (always true for now)
}

/// Authentication filter with role requirement for secure endpoints
pub fn with_auth_and_role(role: &'static str) -> impl Filter<Extract = (AuthInfo,), Error = warp::Rejection> + Clone {
    with_auth().and_then(move |auth_info: AuthInfo| async move {
        if auth_info.roles.contains(&role.to_string()) {
            Ok(auth_info)
        } else {
            Err(reject_with_api_error(
                ApiError::Forbidden("Insufficient permissions".to_string())
            ))
        }
    })
} 