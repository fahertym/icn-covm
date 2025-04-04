use icn_covm::storage::{StorageBackend, InMemoryStorage};
use icn_covm::vm::{VM, Op};

#[test]
fn test_in_memory_storage() {
    let mut storage = InMemoryStorage::new();
    
    // Test basic operations
    assert!(!storage.contains("key1"));
    
    storage.set("key1", "42.0").unwrap();
    assert!(storage.contains("key1"));
    assert_eq!(storage.get("key1").unwrap(), "42.0");
    
    storage.set("key2", "123.45").unwrap();
    assert_eq!(storage.get("key2").unwrap(), "123.45");
    
    storage.delete("key1").unwrap();
    assert!(!storage.contains("key1"));
    
    let keys = storage.list_keys(None);
    assert_eq!(keys.len(), 1);
    assert!(keys.contains(&"key2".to_string()));
}

#[test]
fn test_in_memory_storage_transaction() {
    let mut storage = InMemoryStorage::new();
    
    // Set initial values
    storage.set("key1", "10.0").unwrap();
    
    // Begin transaction
    storage.begin_transaction().unwrap();
    
    // Modify values in transaction
    storage.set("key1", "20.0").unwrap();
    storage.set("key2", "30.0").unwrap();
    
    // Values should reflect transaction changes
    assert_eq!(storage.get("key1").unwrap(), "20.0");
    assert_eq!(storage.get("key2").unwrap(), "30.0");
    
    // Rollback transaction
    storage.rollback_transaction().unwrap();
    
    // Values should be restored to pre-transaction state
    assert_eq!(storage.get("key1").unwrap(), "10.0");
    assert!(storage.get("key2").is_err());
    
    // Begin a new transaction
    storage.begin_transaction().unwrap();
    
    // Modify values again
    storage.set("key1", "50.0").unwrap();
    storage.set("key2", "60.0").unwrap();
    
    // Commit transaction
    storage.commit_transaction().unwrap();
    
    // Values should reflect committed changes
    assert_eq!(storage.get("key1").unwrap(), "50.0");
    assert_eq!(storage.get("key2").unwrap(), "60.0");
}

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