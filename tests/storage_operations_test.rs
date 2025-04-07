use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageBackend;
use icn_covm::vm::{VM, Op};

#[test]
fn test_basic_storage_operations() {
    // Create storage backend and VM
    let mut storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set up auth context with proper roles
    let mut auth = AuthContext::new("test_user");
    auth.add_role("global", "admin"); // Need admin to create accounts
    auth.add_role("test_namespace", "writer"); // Can read and write
    vm.set_auth_context(auth.clone());
    vm.set_namespace("test_namespace");
    
    // Create an account for the test user
    if let Some(storage_backend) = vm.storage_backend.as_mut() {
        storage_backend.create_account(&auth, "test_user", 1000).unwrap();
    }
    
    // Store a value outside of transaction first
    let store_program = vec![
        Op::Push(42.0),
        Op::StoreP("test_key".to_string()),
    ];
    let result = vm.execute(&store_program);
    assert!(result.is_ok(), "Store operation failed: {:?}", result);
    
    // Begin a transaction
    let begin_tx_program = vec![Op::BeginTx];
    let result = vm.execute(&begin_tx_program);
    assert!(result.is_ok(), "Begin transaction failed: {:?}", result);
    
    // Store another value
    let store_program2 = vec![
        Op::Push(100.0),
        Op::StoreP("another_key".to_string()),
    ];
    let result = vm.execute(&store_program2);
    assert!(result.is_ok(), "Second store operation failed: {:?}", result);
    
    // Delete a key
    let delete_program = vec![
        Op::DeleteP("test_key".to_string()),
    ];
    let result = vm.execute(&delete_program);
    assert!(result.is_ok(), "Delete operation failed: {:?}", result);
    
    // Verify key is gone
    let exists_program2 = vec![
        Op::KeyExistsP("test_key".to_string()),
    ];
    let result = vm.execute(&exists_program2);
    assert!(result.is_ok(), "Key exists check failed: {:?}", result);
    assert_eq!(vm.top(), Some(0.0), "Expected key to not exist (0.0)");
    vm.stack.clear();
    
    // Roll back transaction (should restore the deleted key and remove the added key)
    let rollback_program = vec![Op::RollbackTx];
    let result = vm.execute(&rollback_program);
    assert!(result.is_ok(), "Rollback transaction failed: {:?}", result);
    
    // DEBUGGING: Directly access the storage backend and list all keys
    if let Some(storage_backend) = vm.storage_backend.as_ref() {
        println!("===== AFTER ROLLBACK =====");
        let keys = storage_backend.list_keys(&auth, "test_namespace", None).unwrap();
        println!("Keys in storage: {:?}", keys);
        let exists = storage_backend.contains(&auth, "test_namespace", "test_key").unwrap();
        println!("test_key exists directly in backend: {}", exists);
    }
    
    // Verify test_key exists after rollback (it should be restored)
    let exists_program3 = vec![
        Op::KeyExistsP("test_key".to_string()),
    ];
    let result = vm.execute(&exists_program3);
    assert!(result.is_ok(), "Key exists check after rollback failed: {:?}", result);
    println!("KeyExistsP result: {:?}", vm.top());
    assert_eq!(vm.top(), Some(1.0), "Expected key to exist after rollback (1.0)");
    vm.stack.clear();
    
    // Verify the value of the restored key
    let load_after_rollback = vec![
        Op::LoadP("test_key".to_string()),
    ];
    let result = vm.execute(&load_after_rollback);
    assert!(result.is_ok(), "Load after rollback failed: {:?}", result);
    assert_eq!(vm.top(), Some(42.0), "Expected original value 42.0 to be restored");
    vm.stack.clear();
    
    // Verify another_key does not exist after rollback (it was added in transaction)
    let exists_program4 = vec![
        Op::KeyExistsP("another_key".to_string()),
    ];
    let result = vm.execute(&exists_program4);
    assert!(result.is_ok(), "Key exists check after rollback failed: {:?}", result);
    assert_eq!(vm.top(), Some(0.0), "Expected added key to not exist after rollback (0.0)");
}

#[test]
fn test_transaction_operations() {
    // Create storage backend and VM
    let mut storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set up auth context with proper roles
    let mut auth = AuthContext::new("test_user");
    auth.add_role("global", "admin"); // Need admin to create accounts
    auth.add_role("test_namespace", "writer"); // Can read and write
    vm.set_auth_context(auth.clone());
    vm.set_namespace("test_namespace");
    
    // Create an account for the test user
    if let Some(storage_backend) = vm.storage_backend.as_mut() {
        storage_backend.create_account(&auth, "test_user", 1000).unwrap();
    }
    
    // Test begin/commit transaction
    
    // Begin a transaction
    let begin_tx_program = vec![Op::BeginTx];
    let result = vm.execute(&begin_tx_program);
    assert!(result.is_ok(), "Begin transaction failed: {:?}", result);
    
    // Store a value
    let store_program = vec![
        Op::Push(42.0),
        Op::StoreP("test_key".to_string()),
    ];
    let result = vm.execute(&store_program);
    assert!(result.is_ok(), "Store operation failed: {:?}", result);
    
    // Commit transaction
    let commit_program = vec![Op::CommitTx];
    let result = vm.execute(&commit_program);
    assert!(result.is_ok(), "Commit transaction failed: {:?}", result);
    
    // Verify key exists after commit
    let exists_program = vec![
        Op::KeyExistsP("test_key".to_string()),
    ];
    let result = vm.execute(&exists_program);
    assert!(result.is_ok(), "Key exists check after commit failed: {:?}", result);
    assert_eq!(vm.top(), Some(1.0), "Expected key to exist after commit (1.0)");
    vm.stack.clear();
    
    // Test rollback
    
    // Begin another transaction
    let begin_tx_program2 = vec![Op::BeginTx];
    let result = vm.execute(&begin_tx_program2);
    assert!(result.is_ok(), "Begin second transaction failed: {:?}", result);
    
    // Delete the key
    let delete_program = vec![
        Op::DeleteP("test_key".to_string()),
    ];
    let result = vm.execute(&delete_program);
    assert!(result.is_ok(), "Delete operation failed: {:?}", result);
    
    // Verify key is gone (within transaction)
    let exists_program2 = vec![
        Op::KeyExistsP("test_key".to_string()),
    ];
    let result = vm.execute(&exists_program2);
    assert!(result.is_ok(), "Key exists check failed: {:?}", result);
    assert_eq!(vm.top(), Some(0.0), "Expected key to not exist (0.0)");
    vm.stack.clear();
    
    // Roll back transaction
    let rollback_program = vec![Op::RollbackTx];
    let result = vm.execute(&rollback_program);
    assert!(result.is_ok(), "Rollback transaction failed: {:?}", result);
    
    // Verify key exists again
    let exists_program3 = vec![
        Op::KeyExistsP("test_key".to_string()),
    ];
    let result = vm.execute(&exists_program3);
    assert!(result.is_ok(), "Key exists check after rollback failed: {:?}", result);
    assert_eq!(vm.top(), Some(1.0), "Expected key to exist after rollback (1.0)");
}

#[test]
fn test_typed_storage_operations() {
    // Create storage backend and VM
    let mut storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set up auth context with proper roles
    let mut auth = AuthContext::new("test_user");
    auth.add_role("global", "admin"); // Need admin to create accounts
    auth.add_role("test_namespace", "writer"); // Can read and write
    vm.set_auth_context(auth.clone());
    vm.set_namespace("test_namespace");
    
    // Create an account for the test user
    if let Some(storage_backend) = vm.storage_backend.as_mut() {
        storage_backend.create_account(&auth, "test_user", 1000).unwrap();
    }
    
    // Test 1: Store and load integer value
    let store_program = vec![
        Op::Push(42.0),
        Op::StorePTyped {
            key: "int_key".to_string(),
            expected_type: "integer".to_string(),
        },
    ];
    let result = vm.execute(&store_program);
    assert!(result.is_ok(), "Store integer typed operation failed: {:?}", result);
    
    let load_program = vec![
        Op::LoadPTyped {
            key: "int_key".to_string(),
            expected_type: "integer".to_string(),
        },
    ];
    let result = vm.execute(&load_program);
    assert!(result.is_ok(), "Load integer typed operation failed: {:?}", result);
    assert_eq!(vm.top(), Some(42.0), "Expected integer value 42.0");
    vm.stack.clear();
    
    // Test 2: Attempt to store a non-integer with integer type (should fail)
    let invalid_store_program = vec![
        Op::Push(42.5), // Not an integer
        Op::StorePTyped {
            key: "int_key2".to_string(),
            expected_type: "integer".to_string(),
        },
    ];
    let result = vm.execute(&invalid_store_program);
    assert!(result.is_err(), "Expected error when storing float as integer");
    
    // Test 3: Store and load a float as number
    let float_store_program = vec![
        Op::Push(42.5),
        Op::StorePTyped {
            key: "number_key".to_string(),
            expected_type: "number".to_string(),
        },
    ];
    let result = vm.execute(&float_store_program);
    assert!(result.is_ok(), "Store number typed operation failed: {:?}", result);
    
    let float_load_program = vec![
        Op::LoadPTyped {
            key: "number_key".to_string(),
            expected_type: "number".to_string(),
        },
    ];
    let result = vm.execute(&float_load_program);
    assert!(result.is_ok(), "Load number typed operation failed: {:?}", result);
    assert_eq!(vm.top(), Some(42.5), "Expected number value 42.5");
    vm.stack.clear();
    
    // Test 4: Store and load boolean (true)
    let bool_true_store_program = vec![
        Op::Push(1.0),  // non-zero = true in VM convention
        Op::StorePTyped {
            key: "bool_true_key".to_string(),
            expected_type: "boolean".to_string(),
        },
    ];
    let result = vm.execute(&bool_true_store_program);
    assert!(result.is_ok(), "Store boolean (true) operation failed: {:?}", result);
    
    let bool_true_load_program = vec![
        Op::LoadPTyped {
            key: "bool_true_key".to_string(),
            expected_type: "boolean".to_string(),
        },
    ];
    let result = vm.execute(&bool_true_load_program);
    assert!(result.is_ok(), "Load boolean (true) operation failed: {:?}", result);
    assert_eq!(vm.top(), Some(1.0), "Expected boolean value 1.0 (true)");
    vm.stack.clear();
    
    // Test 5: Store and load boolean (false)
    let bool_false_store_program = vec![
        Op::Push(0.0),  // zero = false in VM convention
        Op::StorePTyped {
            key: "bool_false_key".to_string(),
            expected_type: "boolean".to_string(),
        },
    ];
    let result = vm.execute(&bool_false_store_program);
    assert!(result.is_ok(), "Store boolean (false) operation failed: {:?}", result);
    
    let bool_false_load_program = vec![
        Op::LoadPTyped {
            key: "bool_false_key".to_string(),
            expected_type: "boolean".to_string(),
        },
    ];
    let result = vm.execute(&bool_false_load_program);
    assert!(result.is_ok(), "Load boolean (false) operation failed: {:?}", result);
    assert_eq!(vm.top(), Some(0.0), "Expected boolean value 0.0 (false)");
    vm.stack.clear();
    
    // Test 6: Store and load string
    let string_store_program = vec![
        Op::Push(123.0),  // Will be converted to string "123"
        Op::StorePTyped {
            key: "string_key".to_string(),
            expected_type: "string".to_string(),
        },
    ];
    let result = vm.execute(&string_store_program);
    assert!(result.is_ok(), "Store string typed operation failed: {:?}", result);
    
    let string_load_program = vec![
        Op::LoadPTyped {
            key: "string_key".to_string(),
            expected_type: "string".to_string(),
        },
    ];
    let result = vm.execute(&string_load_program);
    assert!(result.is_ok(), "Load string typed operation failed: {:?}", result);
    assert_eq!(vm.top(), Some(3.0), "Expected string length 3.0"); // "123" has length 3
    vm.stack.clear();
    
    // Test 7: Store and load null
    let null_store_program = vec![
        Op::Push(0.0),  // Value will be ignored for null type
        Op::StorePTyped {
            key: "null_key".to_string(),
            expected_type: "null".to_string(),
        },
    ];
    let result = vm.execute(&null_store_program);
    assert!(result.is_ok(), "Store null typed operation failed: {:?}", result);
    
    let null_load_program = vec![
        Op::LoadPTyped {
            key: "null_key".to_string(),
            expected_type: "null".to_string(),
        },
    ];
    let result = vm.execute(&null_load_program);
    assert!(result.is_ok(), "Load null typed operation failed: {:?}", result);
    assert_eq!(vm.top(), Some(0.0), "Expected null value 0.0");
    vm.stack.clear();
}

// Create a fresh test to verify single key storage and rollback
#[test]
fn test_simple_transaction_rollback() {
    // Create storage backend and VM
    let mut storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set up auth context with proper roles
    let mut auth = AuthContext::new("test_user");
    auth.add_role("global", "admin"); // Need admin to create accounts
    auth.add_role("test_namespace", "writer"); // Can read and write
    vm.set_auth_context(auth.clone());
    vm.set_namespace("test_namespace");
    
    // Create an account for the test user
    if let Some(storage_backend) = vm.storage_backend.as_mut() {
        storage_backend.create_account(&auth, "test_user", 1000).unwrap();
    }
    
    // Store a value outside of a transaction
    let initial_store = vec![
        Op::Push(42.0),
        Op::StoreP("test_key".to_string()),
    ];
    vm.execute(&initial_store).expect("Initial store should succeed");
    
    // Verify key exists
    let exists_check = vec![
        Op::KeyExistsP("test_key".to_string()),
    ];
    vm.execute(&exists_check).expect("Initial key exists check should succeed");
    assert_eq!(vm.top(), Some(1.0), "Key should exist after initial store");
    vm.stack.clear();
    
    // Begin a transaction
    vm.execute(&vec![Op::BeginTx]).expect("Begin transaction should succeed");
    
    // Delete the key
    vm.execute(&vec![Op::DeleteP("test_key".to_string())]).expect("Delete should succeed");
    
    // Verify key is gone
    vm.execute(&exists_check).expect("Key exists check after delete should succeed");
    assert_eq!(vm.top(), Some(0.0), "Key should not exist after delete");
    vm.stack.clear();
    
    // Rollback transaction
    vm.execute(&vec![Op::RollbackTx]).expect("Rollback should succeed");
    
    // DEBUGGING: Directly access storage backend
    if let Some(storage_backend) = vm.storage_backend.as_ref() {
        println!("===== AFTER ROLLBACK IN SIMPLE TEST =====");
        let keys = storage_backend.list_keys(&auth, "test_namespace", None).unwrap();
        println!("Keys in storage: {:?}", keys);
        let exists = storage_backend.contains(&auth, "test_namespace", "test_key").unwrap();
        println!("test_key exists directly in backend: {}", exists);
    }
    
    // Verify key is restored
    vm.execute(&exists_check).expect("Key exists check after rollback should succeed");
    println!("KeyExistsP result after rollback: {:?}", vm.top());
    assert_eq!(vm.top(), Some(1.0), "Key should exist after rollback");
    vm.stack.clear();
} 