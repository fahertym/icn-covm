//! Tests for the StorageExtensions trait enhancements

use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::{StorageBackend, StorageExtensions};
use icn_covm::storage::errors::StorageError;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestData {
    id: String,
    name: String,
    values: HashMap<String, i32>,
}

fn create_auth_context() -> AuthContext {
    let mut auth = AuthContext::new("test_user");
    auth.add_role("test_namespace", "admin");
    auth
}

#[test]
fn test_json_authed_methods() {
    let mut storage = InMemoryStorage::new();
    let auth = create_auth_context();
    
    // Create test namespace
    storage.create_namespace(Some(&auth), "test_namespace", 1_000_000, None).unwrap();
    
    // Test data
    let test_data = TestData {
        id: "test1".to_string(),
        name: "Test Object".to_string(),
        values: [("key1".to_string(), 100), ("key2".to_string(), 200)]
            .iter().cloned().collect(),
    };
    
    // Test set_json_authed
    storage.set_json_authed(
        &auth,
        "test_namespace",
        "test_object",
        &test_data,
    ).unwrap();
    
    // Test get_json_authed
    let retrieved: TestData = storage.get_json_authed(
        &auth,
        "test_namespace",
        "test_object",
    ).unwrap();
    
    assert_eq!(retrieved, test_data);
    
    // Test contains_authed
    assert!(storage.contains_authed(
        &auth,
        "test_namespace",
        "test_object",
    ).unwrap());
    
    // Test list_keys_authed
    let keys = storage.list_keys_authed(
        &auth,
        "test_namespace",
        None,
    ).unwrap();
    
    assert!(keys.contains(&"test_object".to_string()));
    
    // Test set_json_versioned
    let updated_data = TestData {
        id: "test1".to_string(),
        name: "Updated Test Object".to_string(),
        values: [("key1".to_string(), 150), ("key3".to_string(), 300)]
            .iter().cloned().collect(),
    };
    
    // First version is 1
    let version = storage.set_json_versioned(
        Some(&auth),
        "test_namespace",
        "versioned_object",
        &test_data,
        None,
    ).unwrap();
    
    assert_eq!(version, 1);
    
    // Update with correct version expectation
    let new_version = storage.set_json_versioned(
        Some(&auth),
        "test_namespace",
        "versioned_object",
        &updated_data,
        Some(1),
    ).unwrap();
    
    assert_eq!(new_version, 2);
    
    // Try to update with incorrect version
    let result = storage.set_json_versioned(
        Some(&auth),
        "test_namespace",
        "versioned_object",
        &test_data,
        Some(1),
    );
    
    assert!(matches!(result, Err(StorageError::VersionConflict { .. })));
    
    // Test delete_authed
    storage.delete_authed(
        &auth,
        "test_namespace",
        "test_object",
    ).unwrap();
    
    assert!(!storage.contains_authed(&auth, "test_namespace", "test_object").unwrap());
} 