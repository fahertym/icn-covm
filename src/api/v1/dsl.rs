use warp::{Filter, Rejection, Reply};
use crate::storage::traits::{StorageBackend, StorageExtensions, AsyncStorageExtensions, JsonStorage};
use crate::vm::VM;
use crate::api::auth::{with_auth, AuthInfo, require_role, with_auth_and_role};
use crate::api::error::{ApiError, not_found, bad_request, internal_error, forbidden};
use crate::api::v1::models::{
    MacroDefinition, MacroListResponse, MacroSummary, CreateMacroRequest,
    PaginationParams, SortParams
};
use crate::api::v1;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use std::fmt::Debug;

/// Get all DSL-related API routes
pub fn get_routes<S>(
    storage: Arc<Mutex<S>>,
    vm: Arc<VM<S>>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone 
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + std::fmt::Debug + 'static
{
    let base = warp::path("v1").and(warp::path("dsl"));

    // Wrap the storage in an Arc to share the same reference
    let storage_arc = Arc::new(storage);
    
    // List macros
    let list = base.and(warp::path("macros"))
        .and(warp::get())
        .and(warp::query::<PaginationParams>())
        .and(warp::query::<SortParams>())
        .and(with_storage(storage_arc.clone()))
        .and(with_auth_and_role("Admin".to_string(), Some("User".to_string())))
        .and_then(list_macros_handler);
    
    // Get macro by ID
    let get = base.and(warp::path("macros"))
        .and(warp::path::param::<String>())
        .and(warp::get())
        .and(with_storage(storage_arc.clone()))
        .and(with_auth_and_role("Admin".to_string(), Some("User".to_string())))
        .and_then(get_macro_handler);
    
    // Create new macro
    let create = base.and(warp::path("macros"))
        .and(warp::post())
        .and(warp::body::json::<CreateMacroRequest>())
        .and(with_storage(storage_arc.clone()))
        .and(with_vm(vm.clone()))
        .and(with_auth_and_role("Admin".to_string(), None))
        .and_then(create_macro_handler);
    
    // Update existing macro
    let update = base.and(warp::path("macros"))
        .and(warp::path::param::<String>())
        .and(warp::put())
        .and(warp::body::json::<CreateMacroRequest>())
        .and(with_storage(storage_arc.clone()))
        .and(with_vm(vm.clone()))
        .and(with_auth_and_role("Admin".to_string(), None))
        .and_then(update_macro_handler);
    
    // Delete macro
    let delete = base.and(warp::path("macros"))
        .and(warp::path::param::<String>())
        .and(warp::delete())
        .and(with_storage(storage_arc.clone()))
        .and(with_auth_and_role("Admin".to_string(), None))
        .and_then(delete_macro_handler);
    
    // Execute macro
    let execute = base.and(warp::path("macros"))
        .and(warp::path::param::<String>())
        .and(warp::path("execute"))
        .and(warp::post())
        .and(warp::body::json::<Value>())
        .and(with_storage(storage_arc.clone()))
        .and(with_vm(vm.clone()))
        .and(with_auth_and_role("Admin".to_string(), Some("User".to_string())))
        .and_then(execute_macro_handler);
    
    list.or(get).or(create).or(update).or(delete).or(execute)
}

fn with_vm<S>(vm: Arc<VM<S>>) -> impl Filter<Extract = (Arc<Mutex<VM<S>>>,), Error = std::convert::Infallible> + Clone 
where
    S: Send + Sync + Debug + 'static,
{
    // Convert the Arc<VM<S>> to Arc<Mutex<VM<S>>>
    let vm_mutex = Arc::new(Mutex::new(vm.as_ref().clone()));
    warp::any().map(move || vm_mutex.clone())
}

fn with_storage<S>(storage: Arc<Arc<Mutex<S>>>) -> impl Filter<Extract = (Arc<Arc<Mutex<S>>>,), Error = std::convert::Infallible> + Clone 
where
    S: Send + Sync + 'static,
{
    warp::any().map(move || storage.clone())
}

// Handler implementations
async fn list_macros_handler(
    pagination: PaginationParams,
    sort: SortParams,
    storage: Arc<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + 'static>>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    // Convert pagination parameters to Option<u32>
    let page = pagination.page.map(|p| p as u32);
    let page_size = pagination.page_size.map(|ps| ps as u32);
    let sort_by = sort.sort_by;
    
    // Use the inner Arc<Mutex<...>> with its AsyncStorageExtensions implementation
    let inner_storage = storage.as_ref();
    
    // Call the method directly on the inner_storage, which implements AsyncStorageExtensions
    let macro_list = inner_storage.list_macros(page, page_size, sort_by, None).await
        .map_err(|e| internal_error(&e.to_string()))?;
    
    Ok(warp::reply::json(&macro_list))
}

async fn get_macro_handler(
    id: String,
    storage: Arc<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + 'static>>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    // Use the inner Arc<Mutex<...>> with its AsyncStorageExtensions implementation
    let inner_storage = storage.as_ref();
    
    // Call the get_macro method directly on inner_storage
    let macro_def = inner_storage.get_macro(&id).await
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
    storage: Arc<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + 'static>>>,
    vm: Arc<Mutex<VM<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + std::fmt::Debug + 'static>>>>>,
    auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    // Validate the DSL code by unwrapping the VM and accessing it directly
    let vm_lock = vm.lock().await;
    let _ = vm_lock.validate_dsl(&create_request.code)
        .map_err(|e| bad_request(&format!("Invalid DSL: {}", e)))?;
    
    // Create a new UUID v4 ID for the macro
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    
    let macro_def = crate::storage::MacroDefinition {
        id: id.clone(),
        name: create_request.name.clone(),
        code: create_request.code.clone(),
        description: create_request.description.clone(),
        created_at: now.clone(),
        updated_at: now,
        category: create_request.category.clone(),
        created_by: Some(auth.user_id.clone()),
        // Store visual representation as JSON if provided - fix by using as_ref().map()
        metadata: create_request.visual_representation.as_ref().map(|vr| json!({
            "visual_representation": vr
        })),
    };
    
    // Use the inner Arc<Mutex<...>> with its AsyncStorageExtensions implementation
    let inner_storage = storage.as_ref();
    
    // Call the save_macro method directly on inner_storage
    inner_storage.save_macro(&macro_def).await
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
    storage: Arc<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + 'static>>>,
    vm: Arc<Mutex<VM<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + std::fmt::Debug + 'static>>>>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    // Validate the DSL code by unwrapping the VM and accessing it directly
    let vm_lock = vm.lock().await;
    let _ = vm_lock.validate_dsl(&update_request.code)
        .map_err(|e| bad_request(&format!("Invalid DSL: {}", e)))?;
    
    // Use the inner Arc<Mutex<...>> with its AsyncStorageExtensions implementation
    let inner_storage = storage.as_ref();
    
    // Call the get_macro method directly on inner_storage
    let existing = inner_storage.get_macro(&id).await
        .map_err(|_| not_found(&format!("Macro with id {} not found", id)))?;
    
    let now = Utc::now().to_rfc3339();
    
    let updated_macro = crate::storage::MacroDefinition {
        id: id.clone(),
        name: update_request.name.clone(),
        code: update_request.code.clone(),
        description: update_request.description.clone(),
        created_at: existing.created_at,
        updated_at: now,
        category: update_request.category.clone(),
        created_by: existing.created_by,
        // Store visual representation as JSON if provided
        metadata: update_request.visual_representation.as_ref().map(|vr| json!({
            "visual_representation": vr
        })),
    };
    
    // Call the save_macro method directly on inner_storage
    inner_storage.save_macro(&updated_macro).await
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
    storage: Arc<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + 'static>>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    // Use the inner Arc<Mutex<...>> with its AsyncStorageExtensions implementation
    let inner_storage = storage.as_ref();
    
    // Call the get_macro method directly on inner_storage
    let _ = inner_storage.get_macro(&id).await
        .map_err(|_| not_found(&format!("Macro with id {} not found", id)))?;
    
    // Call the delete_macro method directly on inner_storage
    inner_storage.delete_macro(&id).await
        .map_err(|e| internal_error(&format!("Failed to delete macro: {}", e)))?;
    
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({"status": "success", "message": "Macro deleted successfully"})),
        warp::http::StatusCode::OK,
    ))
}

async fn execute_macro_handler(
    id: String,
    params: Value,
    storage: Arc<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + 'static>>>,
    vm: Arc<Mutex<VM<Arc<Mutex<impl StorageBackend + StorageExtensions + AsyncStorageExtensions + JsonStorage + Send + Sync + Clone + std::fmt::Debug + 'static>>>>>,
    _auth: AuthInfo,
) -> Result<impl Reply, Rejection> {
    // Use the inner Arc<Mutex<...>> with its AsyncStorageExtensions implementation
    let inner_storage = storage.as_ref();
    
    // Call the get_macro method directly on inner_storage
    let macro_def = inner_storage.get_macro(&id).await
        .map_err(|_| not_found(&format!("Macro with id {} not found", id)))?;
    
    // Execute the macro with the provided parameters by unwrapping the VM and accessing it directly
    let vm_lock = vm.lock().await;
    let result = vm_lock.execute_dsl(&macro_def.code, Some(params))
        .map_err(|e| internal_error(&format!("Failed to execute macro: {}", e)))?;
    
    Ok(warp::reply::json(&result))
} 