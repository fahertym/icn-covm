use chrono::Utc;
use std::error::Error;
use std::fmt::Debug;

use crate::governance::proposal::{Proposal, ProposalStatus};
use crate::storage::auth::AuthContext;
use crate::storage::implementations::in_memory::InMemoryStorage;
use crate::storage::traits::StorageExtensions;
use crate::vm::VM;

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
    
    // Create a new proposal
    let proposal_id = "demo-proposal-001";
    let proposal = Proposal::new(
        proposal_id.to_string(),
        user_id.to_string(),
        Some("governance/logic/repair_budget.dsl".to_string()),
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
    
    // First transition: Draft -> Active
    println!("Transitioning proposal to Active...");
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
    let proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;
    
    if matches!(proposal.status, ProposalStatus::Active) {
        println!("✅ Proposal successfully transitioned to Active");
    } else {
        println!("❌ Failed to transition proposal to Active");
    }
    
    // Second transition: Active -> Voting
    println!("Transitioning proposal to Voting...");
    let mut proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;
    
    proposal.mark_voting();
    
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal.storage_key(),
        &proposal,
    )?;
    
    // Verify transition
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
    
    // Final transition: Voting -> Executed with result
    println!("Transitioning proposal to Executed with result...");
    let mut proposal = storage.get_json::<Proposal>(
        Some(&auth),
        "governance",
        &format!("governance/proposals/{}", proposal_id),
    )?;
    
    proposal.mark_executed("Approved with 15/20 votes".to_string());
    
    storage.set_json(
        Some(&auth),
        "governance",
        &proposal.storage_key(),
        &proposal,
    )?;
    
    // Verify final transition
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
    
    println!("Proposal demo completed successfully!");
    
    Ok(())
}

// Helper to set up storage
fn init_storage<S>(vm: &mut VM<S>, auth: &AuthContext) -> Result<(), Box<dyn Error>> 
where 
    S: StorageExtensions + Clone + Send + Sync + Debug + 'static
{
    if let Some(storage) = vm.storage_backend.as_mut() {
        // Create account and namespace
        storage.create_account(Some(auth), "demo_user", 1024 * 1024)?;
        storage.create_namespace(Some(auth), "governance", 1024 * 1024, None)?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_proposal_lifecycle() {
        let result = run_proposal_demo();
        assert!(result.is_ok());
    }
} 