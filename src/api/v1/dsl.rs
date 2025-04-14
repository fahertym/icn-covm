use warp::{Filter, Rejection, Reply};
use crate::storage::traits::{StorageBackend, StorageExtensions, AsyncStorageExtensions};
use crate::api::storage::AsyncStorage;
use crate::vm::VM;
use crate::api::auth::{with_auth, AuthInfo, require_role};
use crate::api::error::{ApiError, not_found, bad_request, internal_error};
use crate::api::v1::models::{
    MacroDefinition, MacroListResponse, MacroSummary, CreateMacroRequest,
    PaginationParams, SortParams
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;

/// Get all DSL-related API routes
pub fn get_routes<S>(
    storage: Arc<Mutex<S>>,
    vm: Arc<VM<Arc<Mutex<S>>>>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone 
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + Send + Sync + Clone + std::fmt::Debug + 'static
{
    let base = warp::path("dsl");
    
    // Wrap storage in AsyncStorage adapter
    let async_storage = Arc::new(storage.clone());
    
    let list_macros = base
        .and(warp::path("macros"))
        .and(warp::get())
        .and(warp::query::<PaginationParams>())
        .and(warp::query::<SortParams>())
        .and(with_storage(async_storage.clone()))
        .and(with_auth())
        .and_then(list_macros_handler);
    
    let get_macro = base
        .and(warp::path("macros"))
        .and(warp::path::param::<String>())
        .and(warp::get())
        .and(with_storage(async_storage.clone()))
        .and(with_auth())
        .and_then(get_macro_handler);
    
    let create_macro = base
        .and(warp::path("macros"))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_storage(async_storage.clone()))
        .and(with_vm(vm.clone()))
        .and(with_auth())
        .and(warp::any().and_then(|auth: AuthInfo| async move {
            require_role(auth, "dsl:write")
        }))
        .and_then(create_macro_handler);
    
    let update_macro = base
        .and(warp::path("macros"))
        .and(warp::path::param::<String>())
        .and(warp::put())
        .and(warp::body::json())
        .and(with_storage(async_storage.clone()))
        .and(with_vm(vm.clone()))
        .and(with_auth())
        .and(warp::any().and_then(|auth: AuthInfo| async move {
            require_role(auth, "dsl:write")
        }))
        .and_then(update_macro_handler);
    
    let delete_macro = base
        .and(warp::path("macros"))
        .and(warp::path::param::<String>())
        .and(warp::delete())
        .and(with_storage(async_storage.clone()))
        .and(with_auth())
        .and(warp::any().and_then(|auth: AuthInfo| async move {
            require_role(auth, "dsl:write")
        }))
        .and_then(delete_macro_handler);
    
    let execute_macro = base
        .and(warp::path("macros"))
        .and(warp::path::param::<String>())
        .and(warp::path("execute"))
        .and(warp::post())
        .and(warp::body::json())
        .and(with_storage(async_storage.clone()))
        .and(with_vm(vm.clone()))
        .and(with_auth())
        .and(warp::any().and_then(|auth: AuthInfo| async move {
            require_role(auth, "dsl:execute")
        }))
        .and_then(execute_macro_handler);
    
    list_macros
        .or(get_macro)
        .or(create_macro)
        .or(update_macro)
        .or(delete_macro)
        .or(execute_macro)
}

// Filter helpers for dependency injection
fn with_storage<S>(storage: Arc<S>) -> impl Filter<Extract = (Arc<S>,), Error = std::convert::Infallible> + Clone 
where
    S: AsyncStorage + Send + Sync + 'static
{
    warp::any().map(move || storage.clone())
}

fn with_vm<S>(vm: Arc<VM<S>>) -> impl Filter<Extract = (Arc<VM<S>>,), Error = std::convert::Infallible> + Clone 
where
    S: Send + Sync + 'static
{
    warp::any().map(move || vm.clone())
}

// Handler implementations
async fn list_macros_handler(
    pagination: PaginationParams,
    sort: SortParams,
    storage: Arc<impl AsyncStorage>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    let page = pagination.page.unwrap_or(1);
    let page_size = pagination.page_size.unwrap_or(20);
    let sort_by = sort.sort_by.as_deref();
    
    // Get the macros from storage
    let macro_list = storage.list_macros(page, page_size, sort_by, None).await
        .map_err(|e| internal_error(&e.to_string()))?;
    
    Ok(warp::reply::json(&macro_list))
}

async fn get_macro_handler(
    id: String,
    storage: Arc<impl AsyncStorage>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    let macro_def = storage.get_macro(&id).await
        .map_err(|_| not_found(&format!("Macro with id {} not found", id)))?;
    
    // Convert to API model
    let response = MacroDefinition {
        id: macro_def.id,
        name: macro_def.name,
        code: macro_def.code,
        description: macro_def.description,
        created_at: macro_def.created_at,
        updated_at: macro_def.updated_at,
        category: macro_def.category,
        visual_representation: None, // TODO: Extract visual representation if available
    };
    
    Ok(warp::reply::json(&response))
}

async fn create_macro_handler(
    create_request: CreateMacroRequest,
    storage: Arc<impl AsyncStorage>,
    vm: Arc<VM<impl Send + Sync + std::fmt::Debug + 'static>>,
    auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection> {
    // Validate the DSL code
    let _ = vm.validate_dsl(&create_request.code)
        .map_err(|e| bad_request(&format!("Invalid DSL: {}", e)))?;
    
    // Create new macro
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    
    let macro_def = crate::storage::MacroDefinition {
        id: id.clone(),
        name: create_request.name,
        code: create_request.code,
        description: create_request.description,
        created_at: now.clone(),
        updated_at: now,
        category: create_request.category,
        created_by: Some(auth.user_id.clone()),
        // Store visual representation as JSON if provided
        metadata: create_request.visual_representation.map(|vr| json!({
            "visual_representation": vr
        })),
    };
    
    storage.save_macro(&macro_def).await
        .map_err(|e| internal_error(&format!("Failed to save macro: {}", e)))?;
    
    // Return the created macro
    let response = MacroDefinition {
        id: macro_def.id,
        name: macro_def.name,
        code: macro_def.code,
        description: macro_def.description,
        created_at: macro_def.created_at,
        updated_at: macro_def.updated_at,
        category: macro_def.category,
        visual_representation: create_request.visual_representation,
    };
    
    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        warp::http::StatusCode::CREATED,
    ))
}

async fn update_macro_handler(
    id: String,
    update_request: CreateMacroRequest,
    storage: Arc<impl AsyncStorage>,
    vm: Arc<VM<impl Send + Sync + std::fmt::Debug + 'static>>,
    auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection> {
    // Validate the DSL code
    let _ = vm.validate_dsl(&update_request.code)
        .map_err(|e| bad_request(&format!("Invalid DSL: {}", e)))?;
    
    // Check if macro exists
    let existing = storage.get_macro(&id).await
        .map_err(|_| not_found(&format!("Macro with id {} not found", id)))?;
    
    let now = Utc::now().to_rfc3339();
    
    let updated_macro = crate::storage::MacroDefinition {
        id: id.clone(),
        name: update_request.name,
        code: update_request.code,
        description: update_request.description,
        created_at: existing.created_at,
        updated_at: now,
        category: update_request.category,
        created_by: existing.created_by,
        // Store visual representation as JSON if provided
        metadata: update_request.visual_representation.map(|vr| json!({
            "visual_representation": vr
        })),
    };
    
    storage.save_macro(&updated_macro).await
        .map_err(|e| internal_error(&format!("Failed to update macro: {}", e)))?;
    
    // Return the updated macro
    let response = MacroDefinition {
        id: updated_macro.id,
        name: updated_macro.name,
        code: updated_macro.code,
        description: updated_macro.description,
        created_at: updated_macro.created_at,
        updated_at: updated_macro.updated_at,
        category: updated_macro.category,
        visual_representation: update_request.visual_representation,
    };
    
    Ok(warp::reply::json(&response))
}

async fn delete_macro_handler(
    id: String,
    storage: Arc<impl AsyncStorage>,
    _auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection> {
    // Check if macro exists
    let _ = storage.get_macro(&id).await
        .map_err(|_| not_found(&format!("Macro with id {} not found", id)))?;
    
    // Delete the macro
    storage.delete_macro(&id).await
        .map_err(|e| internal_error(&format!("Failed to delete macro: {}", e)))?;
    
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({"status": "success", "message": "Macro deleted successfully"})),
        warp::http::StatusCode::OK,
    ))
}

async fn execute_macro_handler(
    id: String,
    params: Value,
    storage: Arc<impl AsyncStorage>,
    vm: Arc<VM<impl Send + Sync + std::fmt::Debug + 'static>>,
    _auth: AuthInfo,
    _has_role: (),
) -> Result<impl Reply, Rejection> {
    // Get the macro
    let macro_def = storage.get_macro(&id).await
        .map_err(|_| not_found(&format!("Macro with id {} not found", id)))?;
    
    // Execute the macro with the provided parameters
    let result = vm.execute_dsl(&macro_def.code, Some(params))
        .map_err(|e| internal_error(&format!("Failed to execute macro: {}", e)))?;
    
    Ok(warp::reply::json(&result))
} 