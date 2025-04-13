use icn_covm::vm::VM;
use icn_covm::storage::implementations::file_storage::FileStorage;
use icn_covm::storage::auth::AuthContext;
use icn_covm::identity::Identity;
use icn_covm::cli::proposal::{ProposalCli, handle_proposal_command, load_proposal}; // Import ProposalCli and load_proposal
use icn_covm::governance::{ProposalLifecycle, ProposalState, ExecutionStatus}; // Use re-exported types
use icn_covm::storage::traits::StorageBackend; // For direct storage checks

use std::path::PathBuf;
use std::fs;
use clap::CommandFactory; // To build ArgMatches
use tempfile::tempdir; // For temporary storage directory

fn setup_test_vm() -> (VM, AuthContext, PathBuf) {
    // Create a temporary directory for storage
    let dir = tempdir().expect("Failed to create temp dir");
    let storage_path = dir.path().to_path_buf();
    println!("Test storage path: {}", storage_path.display());

    // Create FileStorage backend
    let storage = FileStorage::new(&storage_path).expect("Failed to create FileStorage");

    // Create VM with FileStorage
    let mut vm = VM::with_storage_backend(storage);

    // Create AuthContext using demo helper
    // Note: We need to know/fix the expected DID for 'alice' user if needed.
    // For now, use a placeholder DID and username.
    let alice_did = "did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH"; // Example DID
    let alice_auth = AuthContext::demo_context(alice_did, "alice");

    vm.set_auth_context(alice_auth.clone());
    vm.set_namespace("governance"); // Set default namespace for proposal commands

    (vm, alice_auth, storage_path)
}

// Helper to create AuthContext for another user
fn create_user_auth(user_id: &str) -> AuthContext {
    // Assuming user_id is the public username for demo purposes
    AuthContext::demo_context(&format!("did:key:placeholder-for-{}", user_id), user_id)
}

#[test]
fn test_full_proposal_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let (mut vm, alice_auth, _storage_path) = setup_test_vm();

    // --- 1. Create Proposal --- 
    let cli_app = ProposalCli::command(); // Use imported ProposalCli
    let create_matches = cli_app.get_matches_from(vec![
        "proposal", "create", 
        "--title", "Test Execution Proposal", 
        "--quorum", "2", 
        "--threshold", "2"
    ]);
    
    handle_proposal_command(create_matches.subcommand_matches("proposal").unwrap(), &mut vm, &alice_auth)?;
    println!("Proposal creation command handled.");

    // --- Find the created proposal ID (by listing and picking first/only one) ---
    println!("Attempting to find created proposal ID...");
    let storage = vm.storage_backend.as_ref().ok_or("Storage missing")?;
    let keys = storage.list_keys(Some(&alice_auth), "governance", Some("proposals/"))?;
    println!("Found keys in proposals/: {:?}", keys);
    let proposal_dir_key = keys.iter()
        .find(|k| k.ends_with("/lifecycle") && k.starts_with("proposals/"))
        .ok_or("Proposal lifecycle key not found after creation")?;
    let parts: Vec<&str> = proposal_dir_key.split('/').collect();
    let proposal_id = parts.get(1).ok_or("Could not extract proposal ID from key")?.to_string();
    println!("Assuming created proposal ID: {}", proposal_id);

    // --- 2. Store Execution Logic (Simulating Attachment/Macro) ---
    let logic_dsl = r#"
        StoreP "execution_result" "logic_executed_successfully"
        EmitEvent "governance" "Proposal logic reporting for duty!"
    "#;
    let logic_key = format!("proposals/{}/attachments/logic", proposal_id);
    vm.storage_backend.as_mut().ok_or("Storage missing")?
        .set(Some(&alice_auth), "governance", &logic_key, logic_dsl.as_bytes().to_vec())?;
    println!("Stored execution logic DSL.");

    // --- 3. Publish Proposal ---
    let publish_matches = cli_app.get_matches_from(vec![
        "proposal", "publish", 
        "--id", &proposal_id
    ]);
    handle_proposal_command(publish_matches.subcommand_matches("proposal").unwrap(), &mut vm, &alice_auth)?;
    println!("Proposal published.");

    // --- 4. Start Voting (Implicitly done by publish/transition, but maybe add explicit step later) ---
    // For simplicity, assume publish moves it to a state ready for voting (or directly to Voting)
    // Manually set state to Voting if needed for test
    {
        let mut prop: ProposalLifecycle = load_proposal(&vm, &proposal_id)?;
        // If publish doesn't auto-start voting, do it manually:
        if prop.state != ProposalState::Voting {
            println!("Manually starting voting period for test...");
            prop.start_voting(chrono::Duration::days(1)); // 1 day voting period
             let proposal_bytes = serde_json::to_vec(&prop)?; 
             vm.storage_backend.as_mut().ok_or("Storage missing")?
                 .set(Some(&alice_auth), "governance", &format!("proposals/{}/lifecycle", proposal_id), proposal_bytes)?;
        }
    }

    // --- 5. Cast Votes --- 
    let bob_auth = create_user_auth("bob");
    let charlie_auth = create_user_auth("charlie");

    // Bob votes yes
    let vote_matches_bob = cli_app.get_matches_from(vec![
        "proposal", "vote", 
        "--id", &proposal_id, 
        "--choice", "yes"
    ]);
    // Execute with Bob's auth context
    vm.auth_context = Some(bob_auth.clone());
    handle_proposal_command(vote_matches_bob.subcommand_matches("proposal").unwrap(), &mut vm, &bob_auth)?;
    println!("Bob voted.");

    // Charlie votes yes (this should meet quorum 2, threshold 2)
     let vote_matches_charlie = cli_app.get_matches_from(vec![
        "proposal", "vote", 
        "--id", &proposal_id, 
        "--choice", "yes"
    ]);
     // Execute with Charlie's auth context
    vm.auth_context = Some(charlie_auth.clone());
    handle_proposal_command(vote_matches_charlie.subcommand_matches("proposal").unwrap(), &mut vm, &charlie_auth)?;
    println!("Charlie voted.");

    // --- 6. Verify Final State & Execution --- 
    // Reload proposal with original auth context
    vm.auth_context = Some(alice_auth.clone()); 
    let final_proposal = load_proposal(&vm, &proposal_id)?;

    // Check state
    assert_eq!(final_proposal.state, ProposalState::Executed, "Proposal should be in Executed state");
    println!("Verified final state: Executed");

    // Check execution status
    assert_eq!(final_proposal.execution_status, Some(ExecutionStatus::Success), "Execution status should be Success");
    println!("Verified execution status: Success");

    // Check storage for side effect of execution logic
    let storage = vm.storage_backend.as_ref().ok_or("Storage missing")?;
    let result_bytes = storage.get(Some(&alice_auth), "governance", "execution_result")?;
    let result_string = String::from_utf8(result_bytes)?;
    assert_eq!(result_string, "logic_executed_successfully", "Execution logic side effect not found in storage");
    println!("Verified execution side effect.");

    Ok(())
} 