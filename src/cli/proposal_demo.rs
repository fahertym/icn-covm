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
    let storage = InMemoryStorage::new();
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

    // Store the demo logic in storage - using VM's storage access method 
    let logic_path = "governance/logic/repair_budget.dsl";
    vm.with_storage_mut(|storage| {
        storage.set(
            Some(&auth),
            "governance",
            logic_path,
            logic_content.as_bytes().to_vec(),
        )
    })??;

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

    // Store the proposal using VM's storage access
    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth),
            "governance",
            &proposal.storage_key(),
            &proposal,
        )
    })??;

    println!("Proposal created with ID: {}", proposal_id);

    // Retrieve and verify the proposal using VM's storage access
    let loaded_proposal: Proposal = vm.with_storage(|storage| {
        storage.get_json(
            Some(&auth),
            "governance",
            &proposal.storage_key()
        )
    })??;

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
    let mut proposal = vm.with_storage(|storage| {
        storage.get_json::<Proposal>(
            Some(&auth),
            "governance",
            &proposal.storage_key()
        )
    })??;

    proposal.mark_deliberation();

    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth),
            "governance",
            &proposal.storage_key(),
            &proposal,
        )
    })??;

    // Verify transition
    let proposal = vm.with_storage(|storage| {
        storage.get_json::<Proposal>(
            Some(&auth),
            "governance",
            &proposal.storage_key()
        )
    })??;

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

    // Store the comment using VM's storage access
    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth),
            "governance",
            &format!(
                "governance/proposals/{}/comments/{}",
                proposal_id, comment1_id
            ),
            &comment1,
        )
    })??;

    // Create a reply comment
    let comment2_id = "comment-demo-002";
    let comment2 = crate::cli::proposal::ProposalComment {
        id: comment2_id.to_string(),
        author: user_id.to_string(),
        timestamp: Utc::now(),
        content: "I'm adding a reply to my own comment as an example of threaded discussion."
            .to_string(),
        reply_to: Some(comment1_id.to_string()),
        tags: vec!["example".to_string(), "reply".to_string()],
        reactions: HashMap::new(),
    };

    // Store the reply
    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth),
            "governance",
            &format!(
                "governance/proposals/{}/comments/{}",
                proposal_id, comment2_id
            ),
            &comment2,
        )
    })??;

    println!("Added comments to the proposal");

    // Retrieve and display comments
    let comment_keys = vm.with_storage(|storage| {
        storage.list_keys(
            Some(&auth),
            "governance",
            Some(&format!("governance/proposals/{}/comments", proposal_id)),
        )
    })??;

    println!("Found {} comments", comment_keys.len());

    // Load all comments
    let mut comments = HashMap::new();
    for key in &comment_keys {
        let comment: crate::cli::proposal::ProposalComment = vm.with_storage(|storage| {
            storage.get_json(Some(&auth), "governance", key)
        })??;
        comments.insert(comment.id.clone(), comment);
    }

    println!("Loaded {} comments", comments.len());

    // Display comments in a threaded format
    println!("\n--- Threaded Comments ---");
    let root_comments = comments.values().filter(|c| c.reply_to.is_none());
    for comment in root_comments {
        print_thread_demo(&comments, comment, 0);
    }

    // Transition to voting phase
    println!("\n--- Transitioning to voting phase ---");
    let mut proposal = vm.with_storage(|storage| {
        storage.get_json::<Proposal>(
            Some(&auth),
            "governance",
            &proposal.storage_key()
        )
    })??;

    proposal.mark_voting();

    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth),
            "governance",
            &proposal.storage_key(),
            &proposal,
        )
    })??;

    let proposal = vm.with_storage(|storage| {
        storage.get_json::<Proposal>(
            Some(&auth),
            "governance",
            &proposal.storage_key()
        )
    })??;

    if matches!(proposal.status, ProposalStatus::Voting) {
        println!("✅ Proposal successfully transitioned to Voting");
    } else {
        println!("❌ Failed to transition proposal to Voting");
    }

    // Add votes (simplified for the demo)
    println!("\n--- Recording votes ---");

    // Store votes
    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth),
            "governance",
            &format!("governance/proposals/{}/votes/vote1", proposal_id),
            &serde_json::json!({
                "voter": "member1",
                "vote": "yes",
                "reason": "This budget increase is necessary",
                "timestamp": Utc::now().to_rfc3339()
            }),
        )
    })??;

    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth),
            "governance",
            &format!("governance/proposals/{}/votes/vote2", proposal_id),
            &serde_json::json!({
                "voter": "member2",
                "vote": "yes",
                "reason": "I agree with this proposal",
                "timestamp": Utc::now().to_rfc3339()
            }),
        )
    })??;

    // Count votes for demonstration
    let vote_keys = vm.with_storage(|storage| {
        storage.list_keys(
            Some(&auth),
            "governance",
            Some(&format!("governance/proposals/{}/votes", proposal_id)),
        )
    })??;

    println!("Recorded {} votes", vote_keys.len());

    // Transition to approved
    println!("\n--- Transitioning to approved ---");
    let mut proposal = vm.with_storage(|storage| {
        storage.get_json::<Proposal>(
            Some(&auth),
            "governance",
            &proposal.storage_key()
        )
    })??;

    proposal.mark_approved();

    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth),
            "governance",
            &proposal.storage_key(),
            &proposal,
        )
    })??;

    let proposal = vm.with_storage(|storage| {
        storage.get_json::<Proposal>(
            Some(&auth),
            "governance",
            &proposal.storage_key()
        )
    })??;

    if matches!(proposal.status, ProposalStatus::Approved) {
        println!("✅ Proposal successfully transitioned to Approved");
    } else {
        println!("❌ Failed to transition proposal to Approved");
    }

    // Execute the proposal's DSL logic
    println!("\n--- Executing proposal logic ---");

    // Get the DSL logic
    let logic_path = proposal.logic_path.unwrap();
    let logic_content = vm.with_storage(|storage| {
        storage.get(Some(&auth), "governance", &logic_path)
    })??;
    let logic_str = std::str::from_utf8(&logic_content).unwrap();

    // Parse the DSL
    let (ops, _) = parse_dsl(logic_str)?;

    println!("Executing DSL operations: {} ops", ops.len());

    // Set the namespace and execute
    vm.set_namespace("governance");
    vm.execute(&ops)?;

    println!("Execution result: {:?}", vm.top());

    // Check the stored values from DSL execution
    let budget_value = vm.with_storage(|storage| {
        storage.get(
            Some(&auth),
            "governance",
            "repair_budget",
        )
    })??;
    let budget_str = std::str::from_utf8(&budget_value).unwrap();
    println!("Stored budget value: {}", budget_str);

    println!("\n--- Proposal lifecycle demo completed successfully ---");
    Ok(())
}

/// Helper function to recursively print comments in a threaded format
fn print_thread_demo(
    comments: &HashMap<String, crate::cli::proposal::ProposalComment>,
    comment: &crate::cli::proposal::ProposalComment,
    depth: usize,
) {
    // Print the current comment with indentation
    let indent = "  ".repeat(depth);
    println!(
        "{}{} by {} at {}:",
        indent, comment.id, comment.author, comment.timestamp
    );
    println!("{}{}", indent, comment.content);

    // Print tags if any
    if !comment.tags.is_empty() {
        println!("{}Tags: {}", indent, comment.tags.join(", "));
    }

    // Print reactions if any
    if !comment.reactions.is_empty() {
        let reactions: Vec<String> = comment
            .reactions
            .iter()
            .map(|(reaction, count)| format!("{} ({})", reaction, count))
            .collect();
        println!("{}Reactions: {}", indent, reactions.join(", "));
    }

    println!(""); // Empty line for readability

    // Recursively print replies
    let replies = comments
        .values()
        .filter(|c| c.reply_to.as_ref() == Some(&comment.id));
    for reply in replies {
        print_thread_demo(comments, reply, depth + 1);
    }
}

/// Initialize storage with required namespaces for the proposal demo
fn init_storage<S>(vm: &mut VM<S>, auth: &AuthContext) -> Result<(), Box<dyn Error>>
where
    S: StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    // Create required namespaces using VM's storage access
    vm.with_storage_mut(|storage| {
        storage.create_namespace(Some(auth), "governance", 1_000_000, None)
    })??;
    
    println!("Created governance namespace");
    Ok(())
}

/// Helper function to load DSL code from a file
fn load_dsl_from_file(file_path: &str) -> Result<Vec<Op>, Box<dyn Error>> {
    let content = fs::read_to_string(file_path)?;
    let (ops, _) = parse_dsl(&content)?;
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_lifecycle() {
        assert!(run_proposal_demo().is_ok());
    }
}
