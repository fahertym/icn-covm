use icn_covm::cli::proposal::{handle_proposal_command, load_proposal, ProposalCli}; // Import ProposalCli and load_proposal
use icn_covm::governance::{ExecutionStatus, ProposalLifecycle, ProposalState}; // Use re-exported types
use icn_covm::identity::Identity;
use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::file_storage::FileStorage;
use icn_covm::storage::traits::StorageBackend;
use icn_covm::vm::VM; // For direct storage checks

use clap::CommandFactory; // To build ArgMatches
use std::fs;
use std::path::PathBuf;
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
        "proposal",
        "create",
        "--title",
        "Test Execution Proposal",
        "--quorum",
        "2",
        "--threshold",
        "2",
    ]);

    handle_proposal_command(
        create_matches.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &alice_auth,
    )?;
    println!("Proposal creation command handled.");

    // --- Find the created proposal ID (by listing and picking first/only one) ---
    println!("Attempting to find created proposal ID...");
    let storage = vm.storage_backend.as_ref().ok_or("Storage missing")?;
    let keys = storage.list_keys(Some(&alice_auth), "governance", Some("proposals/"))?;
    println!("Found keys in proposals/: {:?}", keys);
    let proposal_dir_key = keys
        .iter()
        .find(|k| k.ends_with("/lifecycle") && k.starts_with("proposals/"))
        .ok_or("Proposal lifecycle key not found after creation")?;
    let parts: Vec<&str> = proposal_dir_key.split('/').collect();
    let proposal_id = parts
        .get(1)
        .ok_or("Could not extract proposal ID from key")?
        .to_string();
    println!("Assuming created proposal ID: {}", proposal_id);

    // --- 2. Store Execution Logic (Simulating Attachment/Macro) ---
    let logic_dsl = r#"
        StoreP "execution_result" "logic_executed_successfully"
        EmitEvent "governance" "Proposal logic reporting for duty!"
    "#;
    let logic_key = format!("proposals/{}/attachments/logic", proposal_id);
    vm.storage_backend.as_mut().ok_or("Storage missing")?.set(
        Some(&alice_auth),
        "governance",
        &logic_key,
        logic_dsl.as_bytes().to_vec(),
    )?;
    println!("Stored execution logic DSL.");

    // --- 3. Publish Proposal ---
    let publish_matches =
        cli_app.get_matches_from(vec!["proposal", "publish", "--id", &proposal_id]);
    handle_proposal_command(
        publish_matches.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &alice_auth,
    )?;
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
            vm.storage_backend.as_mut().ok_or("Storage missing")?.set(
                Some(&alice_auth),
                "governance",
                &format!("proposals/{}/lifecycle", proposal_id),
                proposal_bytes,
            )?;
        }
    }

    // --- 5. Cast Votes ---
    let bob_auth = create_user_auth("bob");
    let charlie_auth = create_user_auth("charlie");

    // Bob votes yes
    let vote_matches_bob = cli_app.get_matches_from(vec![
        "proposal",
        "vote",
        "--id",
        &proposal_id,
        "--choice",
        "yes",
    ]);
    // Execute with Bob's auth context
    vm.auth_context = Some(bob_auth.clone());
    handle_proposal_command(
        vote_matches_bob.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &bob_auth,
    )?;
    println!("Bob voted.");

    // Charlie votes yes (this should meet quorum 2, threshold 2)
    let vote_matches_charlie = cli_app.get_matches_from(vec![
        "proposal",
        "vote",
        "--id",
        &proposal_id,
        "--choice",
        "yes",
    ]);
    // Execute with Charlie's auth context
    vm.auth_context = Some(charlie_auth.clone());
    handle_proposal_command(
        vote_matches_charlie.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &charlie_auth,
    )?;
    println!("Charlie voted.");

    // --- 6. Verify Final State & Execution ---
    // Reload proposal with original auth context
    vm.auth_context = Some(alice_auth.clone());
    let final_proposal = load_proposal(&vm, &proposal_id)?;

    // Check state
    assert_eq!(
        final_proposal.state,
        ProposalState::Executed,
        "Proposal should be in Executed state"
    );
    println!("Verified final state: Executed");

    // Check execution status
    assert_eq!(
        final_proposal.execution_status,
        Some(ExecutionStatus::Success),
        "Execution status should be Success"
    );
    println!("Verified execution status: Success");

    // Check storage for side effect of execution logic
    let storage = vm.storage_backend.as_ref().ok_or("Storage missing")?;
    let result_bytes = storage.get(Some(&alice_auth), "governance", "execution_result")?;
    let result_string = String::from_utf8(result_bytes)?;
    assert_eq!(
        result_string, "logic_executed_successfully",
        "Execution logic side effect not found in storage"
    );
    println!("Verified execution side effect.");

    Ok(())
}

#[test]
fn test_proposal_quorum_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (mut vm, alice_auth, _storage_path) = setup_test_vm();
    let cli_app = ProposalCli::command();

    // Create proposal (Quorum 2, Threshold 1)
    let create_matches = cli_app.get_matches_from(vec![
        "proposal",
        "create",
        "--title",
        "Quorum Test",
        "--quorum",
        "2",
        "--threshold",
        "1",
    ]);
    handle_proposal_command(
        create_matches.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &alice_auth,
    )?;
    let proposal_id = find_proposal_id(&vm, &alice_auth)?;

    // Publish & start voting (manual state set for simplicity)
    publish_and_start_voting(&mut vm, &alice_auth, &proposal_id)?;

    // Only Bob votes (1 vote < quorum 2)
    let bob_auth = create_user_auth("bob");
    vm.auth_context = Some(bob_auth.clone());
    let vote_matches_bob = cli_app.get_matches_from(vec![
        "proposal",
        "vote",
        "--id",
        &proposal_id,
        "--choice",
        "yes",
    ]);
    handle_proposal_command(
        vote_matches_bob.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &bob_auth,
    )?;

    // Attempt to transition (should reject due to quorum)
    // The transition logic is currently coupled with the vote command, so the last vote triggered it.
    vm.auth_context = Some(alice_auth.clone()); // Switch back to Alice
    let final_proposal = load_proposal(&vm, &proposal_id)?;

    assert_eq!(
        final_proposal.state,
        ProposalState::Rejected,
        "Proposal should be Rejected due to unmet quorum"
    );

    Ok(())
}

#[test]
fn test_proposal_threshold_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (mut vm, alice_auth, _storage_path) = setup_test_vm();
    let cli_app = ProposalCli::command();

    // Create proposal (Quorum 2, Threshold 2)
    let create_matches = cli_app.get_matches_from(vec![
        "proposal",
        "create",
        "--title",
        "Threshold Test",
        "--quorum",
        "2",
        "--threshold",
        "2", // Requires 2 Yes votes
    ]);
    handle_proposal_command(
        create_matches.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &alice_auth,
    )?;
    let proposal_id = find_proposal_id(&vm, &alice_auth)?;

    // Publish & start voting
    publish_and_start_voting(&mut vm, &alice_auth, &proposal_id)?;

    // Bob votes Yes (1 Yes)
    let bob_auth = create_user_auth("bob");
    vm.auth_context = Some(bob_auth.clone());
    let vote_matches_bob = cli_app.get_matches_from(vec![
        "proposal",
        "vote",
        "--id",
        &proposal_id,
        "--choice",
        "yes",
    ]);
    handle_proposal_command(
        vote_matches_bob.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &bob_auth,
    )?;

    // Charlie votes No (1 Yes, 1 No -> Quorum met, Threshold NOT met)
    let charlie_auth = create_user_auth("charlie");
    vm.auth_context = Some(charlie_auth.clone());
    let vote_matches_charlie = cli_app.get_matches_from(vec![
        "proposal",
        "vote",
        "--id",
        &proposal_id,
        "--choice",
        "no",
    ]);
    handle_proposal_command(
        vote_matches_charlie.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &charlie_auth,
    )?;

    // Check final state (should be Rejected)
    vm.auth_context = Some(alice_auth.clone());
    let final_proposal = load_proposal(&vm, &proposal_id)?;
    assert_eq!(
        final_proposal.state,
        ProposalState::Rejected,
        "Proposal should be Rejected due to unmet threshold"
    );

    Ok(())
}

#[test]
fn test_proposal_execution_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (mut vm, alice_auth, _storage_path) = setup_test_vm();
    let cli_app = ProposalCli::command();

    // Create proposal (Quorum 1, Threshold 1)
    let create_matches = cli_app.get_matches_from(vec![
        "proposal",
        "create",
        "--title",
        "Execution Failure Test",
        "--quorum",
        "1",
        "--threshold",
        "1",
    ]);
    handle_proposal_command(
        create_matches.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &alice_auth,
    )?;
    let proposal_id = find_proposal_id(&vm, &alice_auth)?;

    // Store faulty execution logic (division by zero)
    let logic_dsl = "push 1 push 0 div StoreP \"exec_fail_result\" \"should_not_reach\"";
    let logic_key = format!("proposals/{}/attachments/logic", proposal_id);
    vm.storage_backend.as_mut().ok_or("Storage missing")?.set(
        Some(&alice_auth),
        "governance",
        &logic_key,
        logic_dsl.as_bytes().to_vec(),
    )?;

    // Publish & start voting
    publish_and_start_voting(&mut vm, &alice_auth, &proposal_id)?;

    // Bob votes Yes (passes quorum & threshold)
    let bob_auth = create_user_auth("bob");
    vm.auth_context = Some(bob_auth.clone());
    let vote_matches_bob = cli_app.get_matches_from(vec![
        "proposal",
        "vote",
        "--id",
        &proposal_id,
        "--choice",
        "yes",
    ]);
    handle_proposal_command(
        vote_matches_bob.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &bob_auth,
    )?;

    // Check final state (should be Executed, but Failure status)
    vm.auth_context = Some(alice_auth.clone());
    let final_proposal = load_proposal(&vm, &proposal_id)?;
    assert_eq!(
        final_proposal.state,
        ProposalState::Executed,
        "Proposal should be Executed despite logic failure"
    );
    assert!(
        matches!(
            final_proposal.execution_status,
            Some(ExecutionStatus::Failure(_))
        ),
        "Execution status should be Failure"
    );

    // Verify side effect was NOT written (due to rollback)
    let storage = vm.storage_backend.as_ref().ok_or("Storage missing")?;
    match storage.get(Some(&alice_auth), "governance", "exec_fail_result") {
        Err(StorageError::NotFound { .. }) => { /* Expected */ }
        Ok(_) => panic!("Side effect should not have been written after execution failure"),
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

#[test]
fn test_proposal_edit() -> Result<(), Box<dyn std::error::Error>> {
    let (mut vm, alice_auth, _storage_path) = setup_test_vm();
    let cli_app = ProposalCli::command();

    // 1. Create proposal
    let initial_title = "Initial Title";
    let create_matches = cli_app.get_matches_from(vec![
        "proposal",
        "create",
        "--title",
        initial_title,
        "--quorum",
        "1",
        "--threshold",
        "1",
    ]);
    handle_proposal_command(
        create_matches.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &alice_auth,
    )?;
    let proposal_id = find_proposal_id(&vm, &alice_auth)?;

    let proposal_v1 = load_proposal(&vm, &proposal_id)?;
    assert_eq!(proposal_v1.title, initial_title);
    assert_eq!(proposal_v1.current_version, 1);

    // 2. Edit the proposal (e.g., change title via new body attachment)
    let new_title = "Updated Title via Edit";
    let new_body_content = "This is the updated proposal body.";
    let body_file_path = _storage_path.join("new_body.txt");
    fs::write(&body_file_path, new_body_content)?;

    let edit_matches = cli_app.get_matches_from(vec![
        "proposal",
        "edit",
        "--id",
        &proposal_id,
        "--new-body",
        body_file_path.to_str().unwrap(),
        // We need a way to update the title itself directly, or assume editing body implies title change?
        // For now, let's assume edit doesn't change the core lifecycle title field, only attachments.
        // We'll check the attachment content and version bump.
    ]);
    handle_proposal_command(
        edit_matches.subcommand_matches("proposal").unwrap(),
        &mut vm,
        &alice_auth,
    )?;

    // 3. Verify the changes
    let proposal_v2 = load_proposal(&vm, &proposal_id)?;
    // assert_eq!(proposal_v2.title, new_title); // Title field in lifecycle wasn't designed to be edited this way yet
    assert_eq!(
        proposal_v2.current_version, 2,
        "Version should have incremented after edit"
    );

    // Verify new body attachment content
    let storage = vm.storage_backend.as_ref().ok_or("Storage missing")?;
    let body_key = format!("proposals/{}/attachments/body", proposal_id);
    let body_bytes = storage.get(Some(&alice_auth), "governance", &body_key)?;
    assert_eq!(String::from_utf8(body_bytes)?, new_body_content);

    Ok(())
}

// --- Helper Functions Used in Tests ---

// Helper to find the most recently created proposal ID
fn find_proposal_id(
    vm: &VM,
    auth_context: &AuthContext,
) -> Result<String, Box<dyn std::error::Error>> {
    let storage = vm.storage_backend.as_ref().ok_or("Storage missing")?;
    let keys = storage.list_keys(Some(auth_context), "governance", Some("proposals/"))?;
    keys.iter()
        .filter(|k| k.ends_with("/lifecycle") && k.starts_with("proposals/"))
        .max_by_key(|k| {
            // Attempt to parse ID as timestamp for sorting (fragile assumption!)
            k.split('/')
                .nth(1)
                .unwrap_or("")
                .parse::<u64>()
                .unwrap_or(0)
        })
        .map(|k| k.split('/').nth(1).unwrap().to_string())
        .ok_or_else(|| "No proposal lifecycle key found".into())
}

// Helper to publish and manually start voting if needed
fn publish_and_start_voting(
    vm: &mut VM,
    auth_context: &AuthContext,
    proposal_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cli_app = ProposalCli::command();
    let publish_matches =
        cli_app.get_matches_from(vec!["proposal", "publish", "--id", proposal_id]);
    handle_proposal_command(
        publish_matches.subcommand_matches("proposal").unwrap(),
        vm,
        auth_context,
    )?;

    let mut prop = load_proposal(vm, proposal_id)?;
    if prop.state != ProposalState::Voting {
        println!("Manually starting voting period for test...");
        prop.start_voting(chrono::Duration::days(1));
        let proposal_bytes = serde_json::to_vec(&prop)?;
        let lifecycle_key = format!("proposals/{}/lifecycle", proposal_id);
        vm.storage_backend.as_mut().ok_or("Storage missing")?.set(
            Some(auth_context),
            "governance",
            &lifecycle_key,
            proposal_bytes,
        )?;
    }
    Ok(())
}
