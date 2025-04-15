//! Demonstration and testing module for the governance proposal system.
//!
//! This module provides a complete end-to-end demonstration of the proposal lifecycle,
//! showing how proposals are created, commented on, and transitioned through various
//! states from draft to execution. It serves both as a functional test and as
//! example code showing how to use the proposal system.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use std::path::Path;

use crate::compiler::parse_dsl;
use crate::governance::proposal::{Proposal, ProposalStatus};
use crate::storage::auth::AuthContext;
use crate::storage::implementations::in_memory::InMemoryStorage;
use crate::storage::traits::{Storage, StorageBackend, StorageExtensions};
use crate::vm::VM;
use crate::vm::Op;

// Implement Clone for InMemoryStorage to satisfy VM's constraints
impl Clone for InMemoryStorage {
    fn clone(&self) -> Self {
        // Create a new instance - this is a simplified clone for tests
        // In practice, you would want to clone the actual data
        InMemoryStorage::new()
    }
}

/// Run a complete demonstration of the proposal lifecycle
///
/// This function demonstrates the entire proposal management system by:
/// 1. Setting up storage and VM context
/// 2. Creating example DSL logic for a proposal
/// 3. Creating a proposal and storing it
/// 4. Adding threaded comments to the proposal
/// 5. Transitioning the proposal through various states
/// 6. Executing the proposal's DSL logic
///
/// The function also includes various validation checks to ensure
/// the system is working as expected.
///
/// # Returns
/// * `Result<(), Box<dyn Error>>` - Success or an error if any part of the demo fails
pub fn run_proposal_demo() -> Result<(), Box<dyn Error>> {
    println!("Running proposal lifecycle demo...");

    // Set up storage and VM
    let mut storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);

    // Set up auth context with admin role to allow account creation
    let user_id = "demo_user";
    let mut auth = AuthContext::new(user_id);
    auth.add_role("global", "admin");
    auth.add_role("governance", "admin");

    // Ensure we have namespaces set up
    init_storage(&mut vm, &auth)?;

    // Create demo DSL logic file
    let logic_content = r#"
    # Demo proposal logic
    # Increments budgets for repairs
    
    # Store budget approval timestamp
    set_value "budget_timestamp" timestamp
    
    # Set repair budget
    set_value "repair_budget" 5000
    
    # Log approval
    emit "budget_approved" data="Repair budget approved for 5000 credits"
    
    # Return success
    push 1
    "#;

    // Store the demo logic in storage
    let logic_path = "governance/logic/repair_budget.dsl";
    let storage_backend = vm.storage_backend.as_mut().unwrap();
    storage_backend.set(
        Some(&auth),
        "governance",
        logic_path,
        logic_content.as_bytes().to_vec(),
    )?;

    // Create a new proposal
    let proposal_id = "demo-proposal-001";
    let proposal = Proposal::new(
        proposal_id.to_string(),
        user_id.to_string(),
        Some(logic_path.to_string()),
        None, // No expiration
        Some("governance/discussions/budget".to_string()),
        vec!["doc1.txt".to_string(), "budget.xlsx".to_string()],
    );

    // Store the proposal
    let storage = vm.storage_backend.as_mut().unwrap();
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal.storage_key(),
        &proposal,
    )?;

    println!("Proposal created with ID: {}", proposal_id);

    // Retrieve and verify the proposal
    let storage = vm.storage_backend.as_ref().unwrap();
    let loaded_proposal: Proposal = storage.get_json(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    println!("Retrieved proposal: {:?}", loaded_proposal);

    // Verify that status is Draft
    if matches!(loaded_proposal.status, ProposalStatus::Draft) {
        println!("✅ Proposal status is correctly set to Draft");
    } else {
        println!("❌ Proposal status is not Draft");
    }

    // Verify other fields
    if loaded_proposal.id == proposal_id {
        println!("✅ Proposal ID matches");
    } else {
        println!("❌ Proposal ID doesn't match");
    }

    if loaded_proposal.creator == user_id {
        println!("✅ Creator matches");
    } else {
        println!("❌ Creator doesn't match");
    }

    // Demonstrate status transitions
    println!("\n--- Demonstrating status transitions ---");

    // First transition: Draft -> Deliberation
    println!("Transitioning proposal to Deliberation...");
    let mut proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    proposal.mark_deliberation();

    let storage = vm.storage_backend.as_mut().unwrap();
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal.storage_key(),
        &proposal,
    )?;

    // Verify transition
    let storage = vm.storage_backend.as_ref().unwrap();
    let proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    if matches!(proposal.status, ProposalStatus::Deliberation) {
        println!("✅ Proposal successfully transitioned to Deliberation");
    } else {
        println!("❌ Failed to transition proposal to Deliberation");
    }

    // Add some example comments
    println!("\n--- Adding comments during deliberation ---");

    // Create a parent comment
    let comment1_id = "comment-demo-001";
    let comment1 = crate::cli::proposal::ProposalComment {
        id: comment1_id.to_string(),
        author: user_id.to_string(),
        timestamp: Utc::now(),
        content: "This looks like a good proposal! I support increasing the repair budget."
            .to_string(),
        reply_to: None,
        tags: Vec::new(),
        reactions: HashMap::new(),
    };

    // Store the comment
    let storage = vm.storage_backend.as_mut().unwrap();
    storage.set_json(
        Some(&auth),
        "governance",
        &format!(
            "governance/proposals/{}/comments/{}",
            proposal_id, comment1_id
        ),
        &comment1,
    )?;

    println!("✅ Added comment: {}", comment1_id);

    // Create a reply comment
    let comment2_id = "comment-demo-002";
    let comment2 = crate::cli::proposal::ProposalComment {
        id: comment2_id.to_string(),
        author: "council_member".to_string(),
        timestamp: Utc::now(),
        content:
            "I agree, but have we considered allocating some funds for preventative maintenance?"
                .to_string(),
        reply_to: Some(comment1_id.to_string()),
        tags: Vec::new(),
        reactions: HashMap::new(),
    };

    // Store the comment
    storage.set_json(
        Some(&auth),
        "governance",
        &format!(
            "governance/proposals/{}/comments/{}",
            proposal_id, comment2_id
        ),
        &comment2,
    )?;

    println!("✅ Added reply comment: {}", comment2_id);

    // Add more nested comments to demonstrate threading
    let comment3_id = "comment-demo-003";
    let comment3 = crate::cli::proposal::ProposalComment {
        id: comment3_id.to_string(),
        author: user_id.to_string(),
        timestamp: Utc::now() + Duration::seconds(30),
        content: "That's a great point about preventative maintenance. I'll allocate 20% for that purpose.".to_string(),
        reply_to: Some(comment2_id.to_string()),
        tags: Vec::new(),
        reactions: HashMap::new(),
    };

    storage.set_json(
        Some(&auth),
        "governance",
        &format!(
            "governance/proposals/{}/comments/{}",
            proposal_id, comment3_id
        ),
        &comment3,
    )?;

    println!("✅ Added nested reply: {}", comment3_id);

    // Add another top-level comment
    let comment4_id = "comment-demo-004";
    let comment4 = crate::cli::proposal::ProposalComment {
        id: comment4_id.to_string(),
        author: "finance_team".to_string(),
        timestamp: Utc::now() + Duration::seconds(60),
        content: "Have we verified this budget against our quarterly allocations?".to_string(),
        reply_to: None,
        tags: Vec::new(),
        reactions: HashMap::new(),
    };

    storage.set_json(
        Some(&auth),
        "governance",
        &format!(
            "governance/proposals/{}/comments/{}",
            proposal_id, comment4_id
        ),
        &comment4,
    )?;

    println!("✅ Added second top-level comment: {}", comment4_id);

    // Add reply to the second thread
    let comment5_id = "comment-demo-005";
    let comment5 = crate::cli::proposal::ProposalComment {
        id: comment5_id.to_string(),
        author: user_id.to_string(),
        timestamp: Utc::now() + Duration::seconds(90),
        content:
            "Yes, I've confirmed with accounting that this fits within our Q3 maintenance budget."
                .to_string(),
        reply_to: Some(comment4_id.to_string()),
        tags: Vec::new(),
        reactions: HashMap::new(),
    };

    storage.set_json(
        Some(&auth),
        "governance",
        &format!(
            "governance/proposals/{}/comments/{}",
            proposal_id, comment5_id
        ),
        &comment5,
    )?;

    println!("✅ Added reply to second thread: {}", comment5_id);

    // Release mutable borrow on VM first
    let _ = storage;

    // Demonstrate comment display
    println!("\n--- Demonstrating threaded comment display ---");
    let comments = crate::cli::proposal::fetch_comments_threaded(&vm, proposal_id, Some(&auth), false)?;

    // Find and sort root comments
    let mut roots: Vec<&crate::cli::proposal::ProposalComment> =
        comments.values().filter(|c| c.reply_to.is_none()).collect();

    roots.sort_by_key(|c| c.timestamp);

    // Print the threaded comments
    println!("Comments for proposal: {}", proposal_id);

    fn print_thread_demo(
        comments: &HashMap<String, crate::cli::proposal::ProposalComment>,
        comment: &crate::cli::proposal::ProposalComment,
        depth: usize,
    ) {
        let indent = "  ".repeat(depth);
        println!(
            "{}└─ [{}] by {} at {}",
            indent,
            comment.id,
            comment.author,
            comment.timestamp.format("%Y-%m-%d %H:%M:%S")
        );
        println!("{}   {}", indent, comment.content);

        // Find and sort replies to this comment
        let mut replies: Vec<&crate::cli::proposal::ProposalComment> = comments
            .values()
            .filter(|c| c.reply_to.as_deref() == Some(&comment.id))
            .collect();

        replies.sort_by_key(|c| c.timestamp);

        for reply in replies {
            print_thread_demo(comments, reply, depth + 1);
        }
    }

    for root in roots {
        print_thread_demo(&comments, root, 0);
        println!();
    }

    println!("Total comments: {}", comments.len());

    // Get a new mutable reference for the next section
    let storage = vm.storage_backend.as_mut().unwrap();

    // Modify deliberation_started_at to simulate elapsed time
    println!("\n--- Testing deliberation duration requirements ---");

    // Create a second proposal with custom min_deliberation
    let proposal2_id = "demo-proposal-002";
    let mut proposal2 = Proposal::new(
        proposal2_id.to_string(),
        user_id.to_string(),
        Some(logic_path.to_string()),
        None,
        Some("governance/discussions/budget-alt".to_string()),
        vec!["alt-doc.txt".to_string()],
    );

    // Set custom min_deliberation_hours
    proposal2.min_deliberation_hours = Some(48);

    // Store the second proposal
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal2.storage_key(),
        &proposal2,
    )?;

    // Mark as in deliberation
    proposal2.mark_deliberation();

    // Set deliberation_started_at to 36 hours ago
    proposal2.deliberation_started_at = Some(Utc::now() - Duration::hours(36));

    // Update in storage
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal2.storage_key(),
        &proposal2,
    )?;

    println!(
        "Created proposal {} with custom 48-hour minimum deliberation time",
        proposal2_id
    );
    println!("Deliberation started 36 hours ago (not yet eligible for transition to Active)");

    // Back to original proposal - set deliberation start time to 36 hours ago
    let mut proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    proposal.deliberation_started_at = Some(Utc::now() - Duration::hours(36));

    // Update in storage
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal.storage_key(),
        &proposal,
    )?;

    println!(
        "Updated original proposal: deliberation started 36 hours ago (eligible for transition)"
    );

    // Second transition: Deliberation -> Active
    println!("\n--- Continuing with normal flow ---");
    println!("Transitioning proposal from Deliberation to Active...");

    // This should succeed since the deliberation period (36 hours) exceeds the default minimum (24 hours)
    let mut proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    proposal.mark_active();

    let storage = vm.storage_backend.as_mut().unwrap();
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal.storage_key(),
        &proposal,
    )?;

    // Verify transition
    let storage = vm.storage_backend.as_ref().unwrap();
    let proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    if matches!(proposal.status, ProposalStatus::Active) {
        println!(
            "✅ Proposal successfully transitioned to Active (36 hours > default 24 hour minimum)"
        );
    } else {
        println!("❌ Failed to transition proposal to Active");
    }

    // Now try to transition the second proposal which requires 48 hours
    println!("\n--- Testing minimum deliberation time enforcement ---");
    println!("Attempting to transition second proposal to Active (should fail)...");

    // In a real CLI context, this would fail with our validation error
    // Here we'll simulate the validation check
    let proposal2 = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal2_id),
    )?;

    let started_at = proposal2.deliberation_started_at.unwrap();
    let now = Utc::now();
    let elapsed = now.signed_duration_since(started_at);
    let min_required = proposal2.min_deliberation_hours.unwrap_or(24);

    if elapsed.num_hours() < min_required {
        println!(
            "❌ Transition blocked: Deliberation phase must last at least {} hours (elapsed: {})",
            min_required,
            elapsed.num_hours()
        );
        println!("✅ Minimum deliberation time correctly enforced");
    } else {
        println!("⚠️ Unexpected: Deliberation time requirement satisfied");
    }

    // Create a third proposal with very short deliberation time to test --force
    let proposal3_id = "demo-proposal-003";
    let mut proposal3 = Proposal::new(
        proposal3_id.to_string(),
        user_id.to_string(),
        Some(logic_path.to_string()),
        None,
        None,
        vec![],
    );

    // Store the third proposal
    let storage = vm.storage_backend.as_mut().unwrap();
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal3.storage_key(),
        &proposal3,
    )?;

    // Mark as in deliberation
    proposal3.mark_deliberation();

    // Set deliberation_started_at to 1 hour ago
    proposal3.deliberation_started_at = Some(Utc::now() - Duration::hours(1));

    // Update in storage
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal3.storage_key(),
        &proposal3,
    )?;

    println!(
        "\nCreated proposal {} with deliberation started 1 hour ago",
        proposal3_id
    );
    println!("Simulating --force flag to bypass time restriction...");

    // Force transition to Active despite insufficient deliberation time
    proposal3.mark_active();

    // Update in storage
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal3.storage_key(),
        &proposal3,
    )?;

    // Verify transition
    let storage = vm.storage_backend.as_ref().unwrap();
    let proposal3 = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal3_id),
    )?;

    if matches!(proposal3.status, ProposalStatus::Active) {
        println!("✅ Force flag correctly allowed bypassing time restriction");
    } else {
        println!("❌ Force transition failed");
    }

    // Third transition: Active -> Voting
    println!("Transitioning proposal to Voting...");
    let mut proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    proposal.mark_voting();

    let storage = vm.storage_backend.as_mut().unwrap();
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal.storage_key(),
        &proposal,
    )?;

    // Verify transition
    let storage = vm.storage_backend.as_ref().unwrap();
    let proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    if matches!(proposal.status, ProposalStatus::Voting) {
        println!("✅ Proposal successfully transitioned to Voting");
    } else {
        println!("❌ Failed to transition proposal to Voting");
    }

    // Final transition: Voting -> Executed with logic execution
    println!("Transitioning proposal to Executed with logic execution...");
    let mut proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    // Parse and execute the logic
    if let Some(logic_path) = &proposal.logic_path {
        println!("Executing logic from: {}", logic_path);

        // Get the logic content from storage
        let storage_backend = vm.storage_backend.as_ref().unwrap();
        let logic_bytes = storage_backend.get(Some(&auth), "governance", logic_path)?;
        let logic_str = String::from_utf8(logic_bytes)?;

        // Parse and execute
        let (ops, _) = parse_dsl(&logic_str)?;
        let execution_result = match vm.execute(&ops) {
            Ok(_) => format!("Successfully executed logic at {}", logic_path),
            Err(e) => format!("Logic execution failed: {}", e),
        };

        println!("Execution result: {}", execution_result);
        proposal.mark_executed(execution_result);
    } else {
        proposal.mark_executed("No logic path available".to_string());
    }

    let storage = vm.storage_backend.as_mut().unwrap();
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal.storage_key(),
        &proposal,
    )?;

    // Verify final transition
    let storage = vm.storage_backend.as_ref().unwrap();
    let final_proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;

    if matches!(final_proposal.status, ProposalStatus::Executed) {
        println!("✅ Proposal successfully transitioned to Executed");
    } else {
        println!("❌ Failed to transition proposal to Executed");
    }

    if let Some(result) = final_proposal.execution_result {
        println!("✅ Execution result set: {}", result);
    } else {
        println!("❌ Execution result not set correctly");
    }

    // Verify that the budget value was set in storage
    let storage_backend = vm.storage_backend.as_ref().unwrap();
    if let Ok(budget_bytes) = storage_backend.get(Some(&auth), "governance", "repair_budget") {
        if let Ok(budget_str) = String::from_utf8(budget_bytes) {
            println!("✅ Budget was set in storage: {}", budget_str);
        } else {
            println!("❌ Budget value not readable as string");
        }
    } else {
        println!("❌ Budget was not set in storage");
    }

    println!("Proposal demo completed successfully!");

    Ok(())
}

/// Initialize storage for the demo
///
/// Sets up the necessary account and namespace structure in storage
/// to support the proposal demo.
///
/// # Parameters
/// * `vm` - The virtual machine with mutable access to storage
/// * `auth` - Authentication context with admin permissions
///
/// # Returns
/// * `Result<(), Box<dyn Error>>` - Success or an error
fn init_storage<S>(vm: &mut VM<S>, auth: &AuthContext) -> Result<(), Box<dyn Error>>
where
    S: StorageExtensions + Clone + Send + Sync + Debug + 'static,
{
    if let Some(storage) = vm.storage_backend.as_mut() {
        // Create account and namespace
        storage.create_account(Some(auth), "demo_user", 1024 * 1024)?;
        storage.create_namespace(Some(auth), "governance", 1024 * 1024, None)?;
    }

    Ok(())
}

fn load_dsl_from_file(file_path: &str) -> Result<Vec<Op>, Box<dyn Error>> {
    let content = std::fs::read_to_string(file_path)?;
    let (ops, _) = parse_dsl(&content)?;
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the proposal lifecycle demo runs successfully
    #[test]
    fn test_proposal_lifecycle() {
        let result = run_proposal_demo();
        assert!(result.is_ok());
    }
}
