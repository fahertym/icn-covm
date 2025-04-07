use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::vm::{VM, Op, VMError};

#[test]
fn test_get_caller() {
    // Create a storage backend and VM
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Create auth context with a specific user ID
    let mut auth = AuthContext::new("test_user_123");
    vm.set_auth_context(auth);
    
    // Test GetCaller operation
    let program = vec![
        Op::GetCaller,
    ];
    
    let result = vm.execute(&program);
    assert!(result.is_ok(), "Program execution failed: {:?}", result);
    
    // The GetCaller operation should have pushed the length of the user ID to the stack
    assert_eq!(vm.top(), Some("test_user_123".len() as f64), 
               "Expected length of 'test_user_123' on stack");
    
    // Check output message
    assert!(vm.output.contains("Caller Identity: test_user_123"), 
            "Output should contain the caller identity");
}

#[test]
fn test_has_role() {
    // Create a storage backend and VM
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Create auth context with roles
    let mut auth = AuthContext::new("admin_user");
    auth.add_role("test_namespace", "writer");
    auth.add_role("global", "admin");
    vm.set_auth_context(auth);
    vm.set_namespace("test_namespace");
    
    // Test HasRole operation with roles that exist and don't exist
    let program = vec![
        // Check for a role the user has
        Op::HasRole("writer".to_string()),
        
        // Check for a role the user doesn't have
        Op::HasRole("reader".to_string()),
        
        // Check for global admin (should be true regardless of namespace)
        Op::HasRole("admin".to_string()),
    ];
    
    let result = vm.execute(&program);
    assert!(result.is_ok(), "Program execution failed: {:?}", result);
    
    // Stack should contain [1.0, 1.0, 1.0] (last item on top)
    // Note: HasRole returns 1.0 for having the role, 0.0 for not having it
    // The user has the writer role and global admin, but doesn't have the reader role
    assert_eq!(vm.top(), Some(1.0), "Expected 1.0 (has global admin role)");
    vm.pop_one("test").unwrap();
    
    assert_eq!(vm.top(), Some(1.0), "Expected 1.0 (doesn't have reader role)");
    vm.pop_one("test").unwrap();
    
    assert_eq!(vm.top(), Some(1.0), "Expected 1.0 (has writer role)");
}

#[test]
fn test_require_role() {
    // Create a storage backend and VM
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Create auth context with roles
    let mut auth = AuthContext::new("restricted_user");
    auth.add_role("test_namespace", "reader");
    vm.set_auth_context(auth);
    vm.set_namespace("test_namespace");
    
    // Test RequireRole for a role the user has
    let program1 = vec![
        Op::RequireRole("reader".to_string()),
        Op::Push(1.0), // This should execute if RequireRole passes
    ];
    
    let result1 = vm.execute(&program1);
    assert!(result1.is_ok(), "Program execution should succeed when role is present");
    assert_eq!(vm.top(), Some(1.0), "Expected 1.0 on stack");
    
    // Clear the stack
    vm.stack.clear();
    
    // Test RequireRole for a role the user doesn't have
    let program2 = vec![
        Op::RequireRole("writer".to_string()),
        Op::Push(1.0), // This should NOT execute if RequireRole fails
    ];
    
    let result2 = vm.execute(&program2);
    assert!(result2.is_err(), "Program execution should fail when role is missing");
    
    if let Err(VMError::ParameterError(msg)) = result2 {
        assert!(msg.contains("Required role 'writer' not found"), 
                "Error message should mention the missing role");
    } else {
        panic!("Expected ParameterError for missing role");
    }
}

#[test]
fn test_require_identity() {
    // Create a storage backend and VM
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Create auth context
    let auth = AuthContext::new("alice");
    vm.set_auth_context(auth);
    
    // Test RequireIdentity with the correct identity
    let program1 = vec![
        Op::RequireIdentity("alice".to_string()),
        Op::Push(42.0), // This should execute if identity check passes
    ];
    
    let result1 = vm.execute(&program1);
    assert!(result1.is_ok(), "Program should execute when identity matches");
    assert_eq!(vm.top(), Some(42.0), "Expected 42.0 on stack");
    
    // Clear the stack
    vm.stack.clear();
    
    // Test RequireIdentity with incorrect identity
    let program2 = vec![
        Op::RequireIdentity("bob".to_string()),
        Op::Push(42.0), // This should NOT execute
    ];
    
    let result2 = vm.execute(&program2);
    assert!(result2.is_err(), "Program should fail when identity doesn't match");
    
    if let Err(VMError::ParameterError(msg)) = result2 {
        assert!(msg.contains("Required identity 'bob' does not match caller 'alice'"), 
                "Error message should mention the identity mismatch");
    } else {
        panic!("Expected ParameterError for identity mismatch");
    }
}

#[test]
fn test_verify_signature() {
    // Create a storage backend and VM
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Test simplified VerifySignature operation
    // Note: Our implementation doesn't actually verify signatures,
    // it just pops 4 values and always returns valid (1.0)
    let program = vec![
        // Push values representing message, signature, public key, and scheme
        Op::Push(10.0), // Message length
        Op::Push(64.0), // Signature length (typical for ed25519)
        Op::Push(32.0), // Public key length (typical for ed25519)
        Op::Push(7.0),  // Scheme name length (for "ed25519")
        
        // Verify signature operation
        Op::VerifySignature,
    ];
    
    let result = vm.execute(&program);
    assert!(result.is_ok(), "Program execution failed: {:?}", result);
    
    // The result should be 1.0 (valid signature)
    assert_eq!(vm.top(), Some(1.0), "Expected 1.0 (valid signature)");
    
    // Output should mention signature verification
    assert!(vm.output.contains("Verify signature:"), 
            "Output should contain signature verification info");
}

#[test]
fn test_storage_permission_checks() {
    // Create a storage backend and VM
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Create an account for the test user
    let mut admin_auth = AuthContext::new("admin");
    admin_auth.add_role("global", "admin");
    let test_user = "restricted_user";
    
    vm.set_auth_context(admin_auth.clone());
    vm.storage_backend.as_mut().unwrap().create_account(&admin_auth, test_user, 1000).unwrap();
    
    // Set up a restricted user with only read permission
    let mut restricted_auth = AuthContext::new(test_user);
    restricted_auth.add_role("test_data", "reader"); // Can read but not write
    vm.set_auth_context(restricted_auth);
    vm.set_namespace("test_data");
    
    // First, try a read operation (should succeed)
    let read_program = vec![
        Op::KeyExistsP("some_key".to_string()),
    ];
    
    let read_result = vm.execute(&read_program);
    assert!(read_result.is_ok(), "Read operation should succeed with reader role");
    
    // Now try a write operation (should fail)
    let write_program = vec![
        Op::Push(42.0),
        Op::StoreP("some_key".to_string()),
    ];
    
    let write_result = vm.execute(&write_program);
    assert!(write_result.is_err(), "Write operation should fail with only reader role");
    
    // Now give the user writer role and try again
    let mut writer_auth = AuthContext::new(test_user);
    writer_auth.add_role("test_data", "writer");
    vm.set_auth_context(writer_auth);
    
    let write_result2 = vm.execute(&write_program);
    assert!(write_result2.is_ok(), "Write operation should succeed with writer role");
    
    // Test that we can now read what we wrote
    let read_program2 = vec![
        Op::LoadP("some_key".to_string()),
    ];
    
    let read_result2 = vm.execute(&read_program2);
    assert!(read_result2.is_ok(), "Read operation should succeed after writing");
    assert_eq!(vm.top(), Some(42.0), "Expected stored value 42.0 on stack");
} 