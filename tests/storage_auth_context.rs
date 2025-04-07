use icn_covm::bytecode::{BytecodeCompiler, BytecodeExecutor};
use icn_covm::storage::auth::AuthContext;
use icn_covm::vm::VM;
use icn_covm::vm::Op;
use icn_covm::compiler::parse_dsl;
use icn_covm::storage::{StorageBackend, InMemoryStorage};
use std::sync::{Arc, Mutex};

#[test]
fn test_storage_with_auth_context() {
    // Create a DSL program that uses storage
    let source = r#"
        push 42
        storep "answer"
        loadp "answer"
        emit "Answer is: "
    "#;

    // Parse and compile the program
    let ops = parse_dsl(source).unwrap();
    let mut compiler = BytecodeCompiler::new();
    let program = compiler.compile(&ops);
    
    // Create a VM with custom auth context and namespace
    let mut vm = VM::new();
    
    // Create admin auth context for account creation
    let mut admin_auth = AuthContext::new("admin");
    admin_auth.add_role("global", "admin");
    
    // Create accounts for alice and bob
    if let Some(storage) = vm.storage_backend.as_mut() {
        storage.create_account(&admin_auth, "alice", 1000).unwrap();
        storage.create_account(&admin_auth, "bob", 1000).unwrap();
    }
    
    // Set up alice's auth context with appropriate roles
    let mut alice_auth = AuthContext::new("alice");
    alice_auth.add_role("alice_data", "writer");
    alice_auth.add_role("alice_data", "reader");
    alice_auth.add_role("alice_data", "admin");
    
    vm.set_auth_context(alice_auth);
    vm.set_namespace("alice_data");
    
    // Create and run executor
    let mut executor = BytecodeExecutor::new(vm, program.instructions);
    let result = executor.execute();
    assert!(result.is_ok());
    
    // Verify the value was stored in alice's namespace
    assert_eq!(executor.vm.top(), Some(42.0));
    
    // Now create another VM with different auth context
    let mut vm2 = VM::new();
    
    // Set up storage backend for vm2 - reuse admin context
    if let Some(storage) = vm2.storage_backend.as_mut() {
        // Create accounts if they don't exist yet
        storage.create_account(&admin_auth, "alice", 1000).ok();
        storage.create_account(&admin_auth, "bob", 1000).ok();
    }
    
    // Set up bob's auth context with appropriate roles
    let mut bob_auth = AuthContext::new("bob");
    bob_auth.add_role("bob_data", "writer");
    bob_auth.add_role("bob_data", "reader");
    bob_auth.add_role("bob_data", "admin");
    
    vm2.set_auth_context(bob_auth);
    vm2.set_namespace("bob_data");
    
    // Store a different value in bob's namespace
    let source2 = r#"
        push 99
        storep "answer"
        loadp "answer"
        emit "Bob's answer is: "
    "#;
    
    let ops2 = parse_dsl(source2).unwrap();
    let program2 = compiler.compile(&ops2);
    
    let mut executor2 = BytecodeExecutor::new(vm2, program2.instructions);
    let result2 = executor2.execute();
    assert!(result2.is_ok());
    
    // Verify bob's value
    assert_eq!(executor2.vm.top(), Some(99.0));
    
    // Now try to access alice's data with bob's context
    let source3 = r#"
        loadp "answer"
        emit "Trying to access alice's answer: "
    "#;
    
    let ops3 = parse_dsl(source3).unwrap();
    let program3 = compiler.compile(&ops3);
    
    // Create bob VM but set namespace to alice_data
    let mut vm3 = VM::new();
    
    // Make sure accounts exist in this VM
    if let Some(storage) = vm3.storage_backend.as_mut() {
        // Create accounts if they don't exist yet 
        storage.create_account(&admin_auth, "alice", 1000).ok();
        storage.create_account(&admin_auth, "bob", 1000).ok();
    }
    
    // Set up bob's auth context but without permissions to alice's data
    let mut bob_auth2 = AuthContext::new("bob");
    bob_auth2.add_role("bob_data", "writer");
    bob_auth2.add_role("bob_data", "reader");
    // Note: No permissions to alice_data
    
    vm3.set_auth_context(bob_auth2);
    vm3.set_namespace("alice_data");
    
    let mut executor3 = BytecodeExecutor::new(vm3, program3.instructions);
    
    // This should fail because bob doesn't have permission to read alice's data
    let result3 = executor3.execute();
    println!("Result of bob accessing alice's data: {:?}", result3);
    assert!(result3.is_err(), "Bob shouldn't be able to access Alice's data without permissions");
}

#[test]
fn test_multi_tenant_storage() {
    // Let's go back to basics with a simpler test
    
    // Just create a single InMemoryStorage that we'll reuse
    let mut storage = InMemoryStorage::new();
    
    // Create admin auth context
    let mut admin_auth = AuthContext::new("admin");
    admin_auth.add_role("global", "admin");
    
    // Create accounts
    storage.create_account(&admin_auth, "coop1_admin", 1000).unwrap();
    storage.create_account(&admin_auth, "coop2_admin", 1000).unwrap();
    storage.create_account(&admin_auth, "coop1_user", 1000).unwrap();
    storage.create_account(&admin_auth, "coop2_user", 1000).unwrap();
    
    // Set up auth contexts
    let mut coop1_admin_auth = AuthContext::new("coop1_admin");
    coop1_admin_auth.add_role("coop1", "admin");
    coop1_admin_auth.add_role("coop1", "writer"); 
    coop1_admin_auth.add_role("coop1", "reader");
    
    let mut coop1_user_auth = AuthContext::new("coop1_user");
    coop1_user_auth.add_role("coop1", "reader");
    coop1_user_auth.add_role("coop1", "member");
    
    let mut coop2_admin_auth = AuthContext::new("coop2_admin");
    coop2_admin_auth.add_role("coop2", "admin");
    coop2_admin_auth.add_role("coop2", "writer");
    coop2_admin_auth.add_role("coop2", "reader");
    
    let mut coop2_user_auth = AuthContext::new("coop2_user");
    coop2_user_auth.add_role("coop2", "reader");
    coop2_user_auth.add_role("coop2", "member");
    
    // Store some data in each namespace
    let coop1_value = vec![1, 2, 3, 4];
    let coop2_value = vec![5, 6, 7, 8];
    
    // Write to coop1 namespace
    storage.set(&coop1_admin_auth, "coop1", "data", coop1_value.clone()).unwrap();
    
    // Write to coop2 namespace
    storage.set(&coop2_admin_auth, "coop2", "data", coop2_value.clone()).unwrap();
    
    // Now try to read data using user accounts
    
    // Coop1 user should be able to read coop1 data
    let coop1_data = storage.get(&coop1_user_auth, "coop1", "data").unwrap();
    assert_eq!(coop1_data, coop1_value);
    
    // Coop2 user should be able to read coop2 data
    let coop2_data = storage.get(&coop2_user_auth, "coop2", "data").unwrap();
    assert_eq!(coop2_data, coop2_value);
    
    // Coop1 user should NOT be able to read coop2 data
    assert!(storage.get(&coop1_user_auth, "coop2", "data").is_err());
    
    // Coop2 user should NOT be able to read coop1 data
    assert!(storage.get(&coop2_user_auth, "coop1", "data").is_err());
    
    // But global admin should be able to read both
    let global_coop1_data = storage.get(&admin_auth, "coop1", "data").unwrap();
    assert_eq!(global_coop1_data, coop1_value);
    
    let global_coop2_data = storage.get(&admin_auth, "coop2", "data").unwrap();
    assert_eq!(global_coop2_data, coop2_value);
} 