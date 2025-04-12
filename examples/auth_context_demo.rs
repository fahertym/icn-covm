use std::error::Error;
use icn_covm::bytecode::{BytecodeCompiler, BytecodeExecutor};
use icn_covm::compiler::parse_dsl;
use icn_covm::vm::VM;
use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;

fn main() {
    println!("=== ICN-COVM Dynamic Auth Context Demo ===");

    // Create auth contexts for different users
    let mut admin_ctx = AuthContext::new("admin_user");
    admin_ctx.add_role("default", "admin");
    
    let mut member_ctx = AuthContext::new("member_user");
    member_ctx.add_role("default", "member");
    
    let observer_ctx = AuthContext::new("observer_user");
    
    // Create storage
    let storage = InMemoryStorage::default();
    
    println!("1. Setting up governance proposals as admin");
    
    let mut admin_vm = VM::new();
    admin_vm.set_auth_context(admin_ctx);
    
    let source = r#"
        # Set up a new proposal
        push 0
        storep "proposal_001_votes"
        
        push 0.5
        storep "proposal_001_quorum"
        
        push 7
        storep "proposal_001_duration"
        
        # Get values to verify
        loadp "proposal_001_votes"
        emit "Votes: "
        
        loadp "proposal_001_quorum"
        emit "Quorum: "
        
        loadp "proposal_001_duration"
        emit "Duration: "
    "#;
    
    println!("Executing admin operations...");
    match execute_dsl(source, admin_vm) {
        Ok(_) => println!("Admin operations completed successfully"),
        Err(e) => println!("Admin operations failed: {:?}", e),
    }
    
    println!("\n2. Member attempting to read and vote on proposal");
    
    let mut member_vm = VM::new();
    member_vm.set_auth_context(member_ctx);
    
    let source2 = r#"
        # Read proposal info
        loadp "proposal_001_quorum"
        emit "Quorum: "
        
        # Cast a vote
        push 1
        storep "proposal_001_vote_member_user"
        
        # Verify the vote was stored
        loadp "proposal_001_vote_member_user"
        emit "Member vote: "
    "#;
    
    println!("Executing member operations...");
    match execute_dsl(source2, member_vm) {
        Ok(_) => println!("Member operations completed successfully"),
        Err(e) => println!("Member operations failed: {:?}", e),
    }
    
    println!("Observer has admin role: {}", observer_ctx.has_role("default", "admin"));
    println!("Observer has member role: {}", observer_ctx.has_role("default", "member"));
    
    println!("\n3. Observer attempting to access data");
    
    let mut observer_vm = VM::new();
    observer_vm.set_auth_context(observer_ctx);
    
    let source3 = r#"
        # Try to read proposal info
        loadp "proposal_001_quorum"
        emit "Observer trying to read quorum: "
    "#;
    
    println!("Executing observer operations...");
    match execute_dsl(source3, observer_vm) {
        Ok(_) => println!("Observer operations completed successfully (unexpected)"),
        Err(e) => println!("Observer operations failed as expected due to permissions: {:?}", e),
    }
    
    println!("\n=== Demo completed ===");
}

fn execute_dsl(source: &str, vm: VM) -> Result<(), Box<dyn Error>> {
    // Parse DSL
    let ops = parse_dsl(source)?;
    
    // Compile to bytecode
    let mut compiler = BytecodeCompiler::new();
    let program = compiler.compile(&ops);
    
    // Create executor with configured VM
    let mut executor = BytecodeExecutor::new(vm, program.instructions);
    
    // Execute the program
    executor.execute()?;
    
    Ok(())
} 