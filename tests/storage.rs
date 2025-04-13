use icn_covm::storage::errors::StorageResult;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageBackend;

mod test_helpers;
use test_helpers::{create_admin_auth, from_bytes, to_bytes};

#[test]
fn test_in_memory_storage() -> StorageResult<()> {
    let mut storage = InMemoryStorage::new();
    let admin = create_admin_auth();

    // Setup a namespace for testing
    storage.create_namespace(Some(&admin), "test", 1024 * 1024, None)?;
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;

    // Test basic operations
    assert!(!storage.get(Some(&admin), "test", "key1").is_ok());

    storage.set(Some(&admin), "test", "key1", to_bytes("42.0"))?;
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key1")?),
        "42.0"
    );

    storage.set(Some(&admin), "test", "key2", to_bytes("123.45"))?;
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key2")?),
        "123.45"
    );

    storage.delete(Some(&admin), "test", "key1")?;
    assert!(storage.get(Some(&admin), "test", "key1").is_err());

    let keys = storage.list_keys(Some(&admin), "test", None)?;
    assert_eq!(keys.len(), 1);
    assert!(keys.contains(&"key2".to_string()));

    Ok(())
}

#[test]
fn test_in_memory_storage_transaction() -> StorageResult<()> {
    let mut storage = InMemoryStorage::new();
    let admin = create_admin_auth();

    // Setup a namespace for testing
    storage.create_namespace(Some(&admin), "test", 1024 * 1024, None)?;
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;

    // Set initial values
    storage.set(Some(&admin), "test", "key1", to_bytes("10.0"))?;

    // Begin transaction
    storage.begin_transaction()?;

    // Modify values in transaction
    storage.set(Some(&admin), "test", "key1", to_bytes("20.0"))?;
    storage.set(Some(&admin), "test", "key2", to_bytes("30.0"))?;

    // Values should reflect transaction changes
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key1")?),
        "20.0"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key2")?),
        "30.0"
    );

    // Rollback transaction
    storage.rollback_transaction()?;

    // Values should be restored to pre-transaction state
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key1")?),
        "10.0"
    );
    assert!(storage.get(Some(&admin), "test", "key2").is_err());

    // Begin a new transaction
    storage.begin_transaction()?;

    // Modify values again
    storage.set(Some(&admin), "test", "key1", to_bytes("50.0"))?;
    storage.set(Some(&admin), "test", "key2", to_bytes("60.0"))?;

    // Commit transaction
    storage.commit_transaction()?;

    // Values should reflect committed changes
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key1")?),
        "50.0"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key2")?),
        "60.0"
    );

    Ok(())
}

// NOTE: The VM storage integration tests would need to be updated to work with the new storage API
// For now, commenting them out until the VM component is updated to handle AuthContext and namespaces

/*
#[test]
fn test_vm_storage_integration() {
    let mut vm = VM::new();

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
*/
