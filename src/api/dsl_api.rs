use crate::compiler::macros;
use crate::storage::traits::{Storage, StorageExtensions};
use crate::vm::VM;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{Filter, Rejection, Reply};

/// Response for DSL macro listing
#[derive(Debug, Serialize, Deserialize)]
pub struct MacroListResponse {
    macros: Vec<MacroInfo>,
}

/// Information about a single macro
#[derive(Debug, Serialize, Deserialize)]
pub struct MacroInfo {
    id: String,
    name: String,
    description: Option<String>, 
    created_at: String,
    updated_at: String,
    category: Option<String>,
}

/// Response for macro details including code
#[derive(Debug, Serialize, Deserialize)]
pub struct MacroDetailsResponse {
    id: String,
    name: String,
    code: String,
    description: Option<String>,
    created_at: String,
    updated_at: String,
    category: Option<String>,
    visual_representation: Option<MacroVisualRepresentation>,
}

/// Visual representation of a macro for the UI
#[derive(Debug, Serialize, Deserialize)]
pub struct MacroVisualRepresentation {
    nodes: Vec<NodeInfo>,
    edges: Vec<EdgeInfo>,
}

/// Node info for visual representation
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    id: String,
    node_type: String,
    data: HashMap<String, serde_json::Value>,
    position: Position,
}

/// Position data for a node
#[derive(Debug, Serialize, Deserialize)]
pub struct Position {
    x: f64,
    y: f64,
}

/// Edge info for visual representation
#[derive(Debug, Serialize, Deserialize)]
pub struct EdgeInfo {
    id: String,
    source: String,
    target: String,
    animated: Option<bool>,
    label: Option<String>,
}

/// Request to save a macro
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveMacroRequest {
    name: String,
    code: String,
    description: Option<String>,
    category: Option<String>,
    visual_representation: Option<MacroVisualRepresentation>,
}

/// Returns all the DSL API routes
pub fn get_routes<S>(vm: VM<S>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm = Arc::new(Mutex::new(vm));

    // Route to list all macros
    let list_macros_route = warp::path!("api" / "dsl" / "macros")
        .and(warp::get())
        .and(with_vm(vm.clone()))
        .and_then(list_macros);

    // Route to get a specific macro by name
    let get_macro_route = warp::path!("api" / "dsl" / "macros" / String)
        .and(warp::get())
        .and(with_vm(vm.clone()))
        .and_then(get_macro);

    // Route to save a macro
    let save_macro_route = warp::path!("api" / "dsl" / "macros")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_vm(vm.clone()))
        .and_then(save_macro);

    // Route to delete a macro
    let delete_macro_route = warp::path!("api" / "dsl" / "macros" / String)
        .and(warp::delete())
        .and(with_vm(vm.clone()))
        .and_then(delete_macro);

    // Combine all DSL routes
    list_macros_route
        .or(get_macro_route)
        .or(save_macro_route)
        .or(delete_macro_route)
}

/// Dependency injection helper for the VM
fn with_vm<S>(
    vm: Arc<Mutex<VM<S>>>,
) -> impl Filter<Extract = (Arc<Mutex<VM<S>>>,), Error = Infallible> + Clone
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    warp::any().map(move || vm.clone())
}

/// Handler for GET /api/dsl/macros
async fn list_macros<S>(vm: Arc<Mutex<VM<S>>>) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm_lock = vm.lock().await;
    
    // In a real implementation, we would search for macros in storage
    // For now, let's return a sample list
    let macros = vec![
        MacroInfo {
            id: "macro1".to_string(),
            name: "IncrementBalance".to_string(),
            description: Some("Increases the balance of an account".to_string()),
            created_at: "2023-06-01T12:00:00Z".to_string(),
            updated_at: "2023-06-01T12:00:00Z".to_string(),
            category: Some("economic".to_string()),
        },
        MacroInfo {
            id: "macro2".to_string(),
            name: "CreateProposal".to_string(),
            description: Some("Creates a new governance proposal".to_string()),
            created_at: "2023-06-05T14:30:00Z".to_string(),
            updated_at: "2023-06-05T14:30:00Z".to_string(),
            category: Some("governance".to_string()),
        },
    ];
    
    let response = MacroListResponse { macros };
    Ok(warp::reply::json(&response))
}

/// Handler for GET /api/dsl/macros/{name}
async fn get_macro<S>(name: String, vm: Arc<Mutex<VM<S>>>) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm_lock = vm.lock().await;
    
    // In a real implementation, we would fetch the macro from storage
    // For now, let's return a sample macro
    let macro_code = if name == "IncrementBalance" {
        r#"
# Increment Balance Macro
# Increases an account balance by the specified amount

Push 1.0
Store "amount"
Load "balance"
Load "amount"
Add
Store "balance"
EmitEvent "economic" "Balance updated"
        "#.trim().to_string()
    } else {
        "# Empty macro".to_string()
    };
    
    let response = MacroDetailsResponse {
        id: format!("macro-{}", name.to_lowercase()),
        name,
        code: macro_code,
        description: Some("A sample macro implementation".to_string()),
        created_at: "2023-06-01T12:00:00Z".to_string(),
        updated_at: "2023-06-01T12:00:00Z".to_string(),
        category: Some("economic".to_string()),
        visual_representation: Some(MacroVisualRepresentation {
            nodes: vec![
                NodeInfo {
                    id: "node1".to_string(),
                    node_type: "dslNode".to_string(),
                    data: {
                        let mut data = HashMap::new();
                        data.insert("label".to_string(), serde_json::json!("Push"));
                        data.insert("value".to_string(), serde_json::json!("1.0"));
                        data
                    },
                    position: Position { x: 100.0, y: 100.0 },
                },
                NodeInfo {
                    id: "node2".to_string(),
                    node_type: "dslNode".to_string(),
                    data: {
                        let mut data = HashMap::new();
                        data.insert("label".to_string(), serde_json::json!("Store"));
                        data.insert("value".to_string(), serde_json::json!("amount"));
                        data
                    },
                    position: Position { x: 100.0, y: 200.0 },
                },
            ],
            edges: vec![
                EdgeInfo {
                    id: "edge1".to_string(),
                    source: "node1".to_string(),
                    target: "node2".to_string(),
                    animated: Some(true),
                    label: None,
                },
            ],
        }),
    };
    
    Ok(warp::reply::json(&response))
}

/// Handler for POST /api/dsl/macros
async fn save_macro<S>(
    request: SaveMacroRequest,
    vm: Arc<Mutex<VM<S>>>,
) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm_lock = vm.lock().await;
    
    // In a real implementation, we would save the macro to storage
    // For now, let's just echo back the sent data with an ID
    
    let response = MacroDetailsResponse {
        id: format!("macro-{}", request.name.to_lowercase()),
        name: request.name,
        code: request.code,
        description: request.description,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        category: request.category,
        visual_representation: request.visual_representation,
    };
    
    Ok(warp::reply::json(&response))
}

/// Handler for DELETE /api/dsl/macros/{name}
async fn delete_macro<S>(name: String, vm: Arc<Mutex<VM<S>>>) -> Result<impl Reply, Rejection>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let vm_lock = vm.lock().await;
    
    // In a real implementation, we would delete the macro from storage
    // For now, let's just return a success message
    
    let response = serde_json::json!({
        "success": true,
        "message": format!("Macro '{}' deleted successfully", name)
    });
    
    Ok(warp::reply::json(&response))
} 