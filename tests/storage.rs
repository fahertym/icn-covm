use icn_covm::storage::{StorageBackend, InMemoryStorage, auth::AuthContext};
use icn_covm::vm::{VM, Op};
use std::str;

#[test]
fn test_in_memory_storage() {
    let mut storage = InMemoryStorage::new();
    
    // Create an admin auth context
    let mut auth = AuthContext::new("test_admin");
    auth.add_role("global", "admin");
    auth.add_role("default", "writer");
    auth.add_role("default", "reader");
    
    // Create user account
    storage.create_account(&auth, "test_admin", 1000).unwrap();
    
    // Test basic operations
    let contains_result = storage.contains(&auth, "default", "key1").unwrap();
    assert!(!contains_result, "Expected key not to exist");
    
    storage.set(&auth, "default", "key1", "42.0".as_bytes().to_vec()).unwrap();
    let contains_result = storage.contains(&auth, "default", "key1").unwrap();
    assert!(contains_result, "Expected key to exist");
    
    let value = storage.get(&auth, "default", "key1").unwrap();
    let value_str = str::from_utf8(&value).unwrap();
    assert_eq!(value_str, "42.0");
    
    storage.set(&auth, "default", "key2", "123.45".as_bytes().to_vec()).unwrap();
    let value2 = storage.get(&auth, "default", "key2").unwrap();
    let value2_str = str::from_utf8(&value2).unwrap();
    assert_eq!(value2_str, "123.45");
    
    storage.delete(&auth, "default", "key1").unwrap();
    let contains_result = storage.contains(&auth, "default", "key1").unwrap();
    assert!(!contains_result, "Expected key to be deleted");
    
    let keys = storage.list_keys(&auth, "default", None).unwrap();
    assert_eq!(keys.len(), 1);
    assert!(keys.contains(&"key2".to_string()));
}

#[test]
fn test_in_memory_storage_transaction() {
    let mut storage = InMemoryStorage::new();
    
    // Create an admin auth context
    let mut auth = AuthContext::new("test_admin");
    auth.add_role("global", "admin");
    auth.add_role("default", "writer");
    auth.add_role("default", "reader");
    
    // Create user account
    storage.create_account(&auth, "test_admin", 1000).unwrap();
    
    // Set initial values
    storage.set(&auth, "default", "key1", "10.0".as_bytes().to_vec()).unwrap();
    
    // Begin transaction
    storage.begin_transaction().unwrap();
    
    // Modify values in transaction
    storage.set(&auth, "default", "key1", "20.0".as_bytes().to_vec()).unwrap();
    storage.set(&auth, "default", "key2", "30.0".as_bytes().to_vec()).unwrap();
    
    // Values should reflect transaction changes
    let value1 = storage.get(&auth, "default", "key1").unwrap();
    let value1_str = str::from_utf8(&value1).unwrap();
    assert_eq!(value1_str, "20.0");
    
    let value2 = storage.get(&auth, "default", "key2").unwrap();
    let value2_str = str::from_utf8(&value2).unwrap();
    assert_eq!(value2_str, "30.0");
    
    // Rollback transaction
    storage.rollback_transaction().unwrap();
    
    // Values should be restored to pre-transaction state
    let value1 = storage.get(&auth, "default", "key1").unwrap();
    let value1_str = str::from_utf8(&value1).unwrap();
    assert_eq!(value1_str, "10.0");
    
    let result = storage.get(&auth, "default", "key2");
    assert!(result.is_err());
    
    // Begin a new transaction
    storage.begin_transaction().unwrap();
    
    // Modify values again
    storage.set(&auth, "default", "key1", "50.0".as_bytes().to_vec()).unwrap();
    storage.set(&auth, "default", "key2", "60.0".as_bytes().to_vec()).unwrap();
    
    // Commit transaction
    storage.commit_transaction().unwrap();
    
    // Values should reflect committed changes
    let value1 = storage.get(&auth, "default", "key1").unwrap();
    let value1_str = str::from_utf8(&value1).unwrap();
    assert_eq!(value1_str, "50.0");
    
    let value2 = storage.get(&auth, "default", "key2").unwrap();
    let value2_str = str::from_utf8(&value2).unwrap();
    assert_eq!(value2_str, "60.0");
}

#[test]
fn test_vm_storage_integration() {
    let mut vm = VM::new();
    
    // Set up auth context with proper roles
    let mut auth = AuthContext::new("test_user");
    auth.add_role("global", "admin");
    auth.add_role("default", "writer");
    auth.add_role("default", "reader");
    vm.set_auth_context(auth.clone());
    
    // Create a user account in the VM's storage
    if let Some(storage_backend) = vm.storage_backend.as_mut() {
        storage_backend.create_account(&auth, "test_user", 1000).unwrap();
    }
    
    // Create a program that uses persistent storage
    let ops = vec![
        // Store a value in persistent storage
        Op::Push(42.0),
        Op::StoreP("test_key".to_string()),
        
        // Load the value back from persistent storage
        Op::LoadP("test_key".to_string()),
    ];
    
    // Execute the program
    assert!(vm.execute(&ops).is_ok());
    
    // Verify the value was loaded onto the stack
    assert_eq!(vm.top(), Some(42.0));
}

#[test]
fn test_vm_storage_persistence() {
    let mut vm = VM::new();
    
    // Set up auth context with proper roles
    let mut auth = AuthContext::new("test_user");
    auth.add_role("global", "admin");
    auth.add_role("default", "writer");
    auth.add_role("default", "reader");
    vm.set_auth_context(auth.clone());
    
    // Create a user account in the VM's storage
    if let Some(storage_backend) = vm.storage_backend.as_mut() {
        storage_backend.create_account(&auth, "test_user", 1000).unwrap();
    }
    
    // First program: store values
    let store_ops = vec![
        Op::Push(123.45),
        Op::StoreP("value1".to_string()),
        Op::Push(678.9),
        Op::StoreP("value2".to_string()),
    ];
    
    assert!(vm.execute(&store_ops).is_ok());
    
    // Clear the stack to ensure we're not just reading old values
    vm.stack.clear();
    
    // Second program: load values
    let load_ops = vec![
        Op::LoadP("value1".to_string()),
        Op::LoadP("value2".to_string()),
    ];
    
    assert!(vm.execute(&load_ops).is_ok());
    
    // Verify both values were loaded correctly
    assert_eq!(vm.stack, vec![123.45, 678.9]);
}

#[test]
fn test_vm_storage_arithmetic() {
    let mut vm = VM::new();
    
    // Set up auth context with proper roles
    let mut auth = AuthContext::new("test_user");
    auth.add_role("global", "admin");
    auth.add_role("default", "writer");
    auth.add_role("default", "reader");
    vm.set_auth_context(auth.clone());
    
    // Create a user account in the VM's storage
    if let Some(storage_backend) = vm.storage_backend.as_mut() {
        storage_backend.create_account(&auth, "test_user", 1000).unwrap();
    }
    
    // Program that uses storage with arithmetic
    let ops = vec![
        // Store initial counter value
        Op::Push(10.0),
        Op::StoreP("counter".to_string()),
        
        // Loop 5 times, incrementing the counter each time
        Op::Loop { 
            count: 5, 
            body: vec![
                // Load current value
                Op::LoadP("counter".to_string()),
                // Add 1
                Op::Push(1.0),
                Op::Add,
                // Store back
                Op::StoreP("counter".to_string()),
            ]
        },
        
        // Load final counter value
        Op::LoadP("counter".to_string()),
    ];
    
    assert!(vm.execute(&ops).is_ok());
    
    // Counter should have increased by 5
    assert_eq!(vm.top(), Some(15.0));
} 