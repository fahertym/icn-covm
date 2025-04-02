use icn_covm::bytecode::{BytecodeCompiler, BytecodeExecutor};
use icn_covm::compiler::parse_dsl;
use icn_covm::storage::auth::AuthContext;
use icn_covm::vm::VM;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("VM Storage Demo");
    println!("==============");
    
    // Create a program that uses storage operations
    let source = r#"
        # Store and read a value using persistent storage
        push 42
        storep "answer"
        loadp "answer"
        emit "The answer is: "
    "#;
    
    // Parse the source into operations
    println!("\nCompiling DSL program...");
    let ops = parse_dsl(source)?;
    let mut compiler = BytecodeCompiler::new();
    let program = compiler.compile(&ops);
    
    // Create a fresh storage backend
    println!("Setting up storage backend...");
    let mut storage = InMemoryStorage::new();
    
    // Create an admin auth context for setting up accounts
    println!("Creating user account...");
    let mut admin_auth = AuthContext::new("admin");
    admin_auth.add_role("global", "admin");
    
    // Create an account for our admin user
    storage.create_account(&admin_auth, "admin", 10000)?;
    
    // Create a VM and set up authentication
    println!("Setting up VM...");
    let mut vm = VM::new();
    // Replace the default storage backend with our configured one
    vm.storage_backend = Some(Box::new(storage));
    
    // Setup auth context with admin role
    let mut auth_context = AuthContext::new("admin");
    auth_context.add_role("default", "admin");
    auth_context.add_role("default", "writer");
    auth_context.add_role("default", "reader");
    
    // Set the auth context and namespace
    vm.set_auth_context(auth_context);
    vm.set_namespace("default");
    
    // Execute the program
    println!("Executing program...");
    let mut executor = BytecodeExecutor::new(vm, program.instructions);
    match executor.execute() {
        Ok(_) => {
            println!("Program executed successfully!");
            if let Some(value) = executor.vm.top() {
                println!("Result: {}", value);
            }
            println!("VM Output: {}", executor.vm.output);
        },
        Err(e) => {
            println!("Error executing program: {}", e);
        }
    }
    
    println!("\nDemo completed!");
    Ok(())
} 