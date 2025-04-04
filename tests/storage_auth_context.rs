use icn_covm::bytecode::{BytecodeCompiler, BytecodeExecutor};
use icn_covm::compiler::parse_dsl;
use icn_covm::storage::auth::AuthContext;
use icn_covm::vm::Op;
use icn_covm::vm::VM;

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
    vm.set_auth_context(AuthContext::with_roles("alice", vec!["admin".to_string()]));
    vm.set_namespace("alice_data");

    // Create and run executor
    let mut executor = BytecodeExecutor::new(vm, program.instructions);
    let result = executor.execute();
    assert!(result.is_ok());

    // Verify the value was stored in alice's namespace
    assert_eq!(executor.vm.top(), Some(42.0));

    // Now create another VM with different auth context
    let mut vm2 = VM::new();
    vm2.set_auth_context(AuthContext::with_roles("bob", vec!["member".to_string()]));
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
    vm3.set_auth_context(AuthContext::with_roles("bob", vec!["member".to_string()]));
    vm3.set_namespace("alice_data");

    let mut executor3 = BytecodeExecutor::new(vm3, program3.instructions);

    // This might fail depending on your storage implementation's permission model
    // If you have proper RBAC, it should fail unless bob has access to alice's data
    let result3 = executor3.execute();
    println!("Result of bob accessing alice's data: {:?}", result3);

    // You can force proper permissions checking by adding this code:
    // let default_ns = "default";
    // let can_access = executor3.vm.auth_context.has_role(default_ns, "admin");
    // println!("Bob has admin role: {}", can_access);
}

#[test]
fn test_multi_tenant_storage() {
    // Create two VMs for different cooperatives
    let mut coop1_vm = VM::new();
    coop1_vm.set_auth_context(AuthContext::with_roles(
        "coop1_admin",
        vec!["admin".to_string()],
    ));
    coop1_vm.set_namespace("coop1");

    let mut coop2_vm = VM::new();
    coop2_vm.set_auth_context(AuthContext::with_roles(
        "coop2_admin",
        vec!["admin".to_string()],
    ));
    coop2_vm.set_namespace("coop2");

    // Store values for both coops
    let store_source = r#"
        push 100
        storep "balance"
    "#;

    let ops = parse_dsl(store_source).unwrap();
    let mut compiler = BytecodeCompiler::new();
    let program = compiler.compile(&ops);

    // Store in coop1
    let mut executor1 = BytecodeExecutor::new(coop1_vm, program.instructions.clone());
    let result1 = executor1.execute();
    assert!(result1.is_ok());

    // Store a different value in coop2
    let mut coop2_vm = VM::new();
    coop2_vm.set_auth_context(AuthContext::with_roles(
        "coop2_admin",
        vec!["admin".to_string()],
    ));
    coop2_vm.set_namespace("coop2");

    let store_source2 = r#"
        push 200
        storep "balance"
    "#;

    let ops2 = parse_dsl(store_source2).unwrap();
    let program2 = compiler.compile(&ops2);

    let mut executor2 = BytecodeExecutor::new(coop2_vm, program2.instructions);
    let result2 = executor2.execute();
    assert!(result2.is_ok());

    // Now retrieve and verify they have different values in isolation
    let load_source = r#"
        loadp "balance"
    "#;

    let load_ops = parse_dsl(load_source).unwrap();
    let load_program = compiler.compile(&load_ops);

    // Check coop1 balance
    let mut coop1_vm = VM::new();
    coop1_vm.set_auth_context(AuthContext::with_roles(
        "coop1_user",
        vec!["member".to_string()],
    ));
    coop1_vm.set_namespace("coop1");

    let mut load_executor1 = BytecodeExecutor::new(coop1_vm, load_program.instructions.clone());
    let load_result1 = load_executor1.execute();
    assert!(load_result1.is_ok());
    assert_eq!(load_executor1.vm.top(), Some(100.0));

    // Check coop2 balance
    let mut coop2_vm = VM::new();
    coop2_vm.set_auth_context(AuthContext::with_roles(
        "coop2_user",
        vec!["member".to_string()],
    ));
    coop2_vm.set_namespace("coop2");

    let mut load_executor2 = BytecodeExecutor::new(coop2_vm, load_program.instructions);
    let load_result2 = load_executor2.execute();
    assert!(load_result2.is_ok());
    assert_eq!(load_executor2.vm.top(), Some(200.0));

    println!("Successfully demonstrated namespace isolation between cooperatives!");
}
