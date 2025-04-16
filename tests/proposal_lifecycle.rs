use std::str::FromStr;
use std::time::Duration;
use chrono::{DateTime, Utc};
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::vm::VM;
use serde_json::{json, Value};
use icn_covm::compiler::parse_dsl;
use icn_covm::storage::traits::Storage;
use std::fs;
use icn_covm::governance::proposal_lifecycle::{ProposalLifecycle, ProposalState, VoteChoice, ExecutionStatus};
use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::errors::{StorageResult, StorageError};
use icn_covm::storage::traits::{StorageBackend, StorageExtensions};
use icn_covm::vm::memory::MemoryScope;
use icn_covm::identity::Identity;
use std::collections::HashMap;
use regex;

#[test]
fn test_proposal_lifecycle() -> anyhow::Result<()> {
    // Setup - create VM with storage backend
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set auth context
    let auth_context = AuthContext::new("test-user");
    vm.set_auth_context(auth_context);
    
    // Define proposal parameters
    let namespace = "test-namespace";
    let proposal_id = "test-proposal";
    let proposal_title = "Test Proposal";
    let proposal_description = "This is a test proposal";
    let min_quorum = 0.5; // 50% quorum
    let vote_threshold = 0.6; // 60% approval threshold
    let min_deliberation_period = 3600; // 1 hour
    
    // Create proposal
    create_proposal(
        &mut vm,
        namespace,
        proposal_id,
        proposal_title,
        proposal_description,
        min_quorum,
        vote_threshold,
        min_deliberation_period,
    )?;
    
    // Check if proposal exists and is in draft state
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("id").unwrap().as_str().unwrap(), proposal_id);
    assert_eq!(proposal.get("title").unwrap().as_str().unwrap(), proposal_title);
    assert_eq!(proposal.get("description").unwrap().as_str().unwrap(), proposal_description);
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "DraftPhase");
    assert_eq!(proposal.get("quorum").unwrap().as_f64().unwrap(), min_quorum);
    assert_eq!(proposal.get("threshold").unwrap().as_f64().unwrap(), vote_threshold);
    
    // Open for feedback
    let result = open_for_feedback(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Check proposal is now in feedback state
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "FeedbackPhase");
    
    // Start voting phase
    let result = start_voting(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Check proposal is now in voting state
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "VotingPhase");
    
    // Cast votes
    cast_votes(
        &mut vm,
        namespace,
        proposal_id,
        &[
            ("voter1", 0.3, true),  // 30% yes
            ("voter2", 0.3, true),  // 30% yes
            ("voter3", 0.2, false), // 20% no
        ],
    )?;
    
    // Verify votes were correctly recorded
    let votes = get_proposal_votes(&vm, namespace, proposal_id)?;
    assert_eq!(votes.len(), 3);
    
    // Calculate vote totals to verify quorum and threshold
    let total_weight: f64 = votes.iter().map(|(_, weight, _)| *weight).sum();
    let yes_votes: f64 = votes.iter().filter(|(_, _, vote)| *vote).map(|(_, weight, _)| *weight).sum();
    
    // Verify quorum and threshold requirements met
    assert!(total_weight >= min_quorum, "Quorum requirement not met");
    assert!(yes_votes / total_weight >= vote_threshold, "Threshold requirement not met");
    
    // Execute proposal
    let result = execute_proposal(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Check proposal is now in execution state
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "ExecutionPhase");
    
    Ok(())
}

#[test]
fn test_failing_proposal_quorum() -> anyhow::Result<()> {
    // Setup - create VM with storage backend
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set auth context
    let auth_context = AuthContext::new("test-user");
    vm.set_auth_context(auth_context);
    
    // Define proposal parameters
    let namespace = "test-namespace";
    let proposal_id = "test-proposal-quorum-fail";
    let proposal_title = "Test Proposal (Quorum Fail)";
    let proposal_description = "This is a test proposal that should fail quorum";
    let min_quorum = 0.6; // 60% quorum
    let vote_threshold = 0.5; // 50% approval threshold
    let min_deliberation_period = 3600; // 1 hour
    
    // Create proposal
    create_proposal(
        &mut vm,
        namespace,
        proposal_id,
        proposal_title,
        proposal_description,
        min_quorum,
        vote_threshold,
        min_deliberation_period,
    )?;
    
    // Open for feedback
    let result = open_for_feedback(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Start voting phase
    let result = start_voting(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Cast votes - not enough for quorum (only 40%)
    cast_votes(
        &mut vm,
        namespace,
        proposal_id,
        &[
            ("voter1", 0.3, true),  // 30% yes
            ("voter2", 0.1, true),  // 10% yes
        ],
    )?;
    
    // Verify votes were correctly recorded
    let votes = get_proposal_votes(&vm, namespace, proposal_id)?;
    assert_eq!(votes.len(), 2);
    
    // Calculate vote totals to check quorum
    let total_weight: f64 = votes.iter().map(|(_, weight, _)| *weight).sum();
    
    // Verify quorum requirement not met
    assert!(total_weight < min_quorum, "Should not have reached quorum");
    
    // Try to execute proposal - should fail due to not meeting quorum
    let result = execute_proposal(&mut vm, namespace, proposal_id)?;
    assert!(!result.get("success").unwrap().as_bool().unwrap());
    
    // Check proposal is still in voting state
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "VotingPhase");
    
    Ok(())
}

#[test]
fn test_proposal_execution_retry_tracking() -> anyhow::Result<()> {
    // Setup - create VM with storage backend
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set auth context
    let auth_context = AuthContext::new("test-user");
    vm.set_auth_context(auth_context);
    
    // Define proposal parameters
    let namespace = "test-namespace";
    let proposal_id = "test-proposal-retry";
    let proposal_title = "Test Proposal Retry";
    let proposal_description = "This proposal tests the retry tracking functionality";
    let min_quorum = 0.5; // 50% quorum
    let vote_threshold = 0.5; // 50% approval threshold
    let min_deliberation_period = 3600; // 1 hour
    
    // Create proposal
    create_proposal(
        &mut vm,
        namespace,
        proposal_id,
        proposal_title,
        proposal_description,
        min_quorum,
        vote_threshold,
        min_deliberation_period,
    )?;
    
    // Open for feedback
    let result = open_for_feedback(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Start voting phase
    let result = start_voting(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Cast votes - enough for quorum and threshold
    cast_votes(
        &mut vm,
        namespace,
        proposal_id,
        &[
            ("voter1", 0.6, true),  // 60% yes
            ("voter2", 0.1, false), // 10% no
        ],
    )?;
    
    // Verify votes were correctly recorded
    let votes = get_proposal_votes(&vm, namespace, proposal_id)?;
    assert_eq!(votes.len(), 2);
    
    // Calculate vote totals
    let total_weight: f64 = votes.iter().map(|(_, weight, _)| *weight).sum();
    let yes_weight: f64 = votes
        .iter()
        .filter(|(_, _, vote)| *vote)
        .map(|(_, weight, _)| *weight)
        .sum();
    
    // Verify quorum and threshold met
    assert!(total_weight >= min_quorum, "Should have reached quorum");
    assert!(yes_weight / total_weight >= vote_threshold, "Should have passed threshold");
    
    // Attempt first execution - should fail but track retry
    let retry_data = r#"{"failOnRetry": true, "maxRetries": 3}"#;
    let result = execute_proposal_with_params(&mut vm, namespace, proposal_id, retry_data)?;
    assert!(!result.get("success").unwrap().as_bool().unwrap());
    
    // Verify retry count has been incremented
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    let retries = proposal.get("execution_retries").unwrap().as_u64().unwrap();
    assert_eq!(retries, 1, "Should have tracked one retry attempt");
    
    // Attempt second execution - should fail but track retry
    let result = execute_proposal_with_params(&mut vm, namespace, proposal_id, retry_data)?;
    assert!(!result.get("success").unwrap().as_bool().unwrap());
    
    // Verify retry count has been incremented
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    let retries = proposal.get("execution_retries").unwrap().as_u64().unwrap();
    assert_eq!(retries, 2, "Should have tracked two retry attempts");
    
    // Attempt third execution - should fail but track retry
    let result = execute_proposal_with_params(&mut vm, namespace, proposal_id, retry_data)?;
    assert!(!result.get("success").unwrap().as_bool().unwrap());
    
    // Verify retry count has been incremented to max
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    let retries = proposal.get("execution_retries").unwrap().as_u64().unwrap();
    assert_eq!(retries, 3, "Should have tracked three retry attempts");
    
    // Attempt fourth execution - should fail due to max retries
    let result = execute_proposal_with_params(&mut vm, namespace, proposal_id, retry_data)?;
    assert!(!result.get("success").unwrap().as_bool().unwrap());
    
    // Verify error message mentions max retries
    let error = result.get("error").unwrap().as_str().unwrap();
    assert!(error.contains("max retries"), "Error should mention max retries: {}", error);
    
    // Verify proposal state is still VotingPhase
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "VotingPhase");
    
    Ok(())
}

#[test]
fn test_failing_proposal_threshold() -> anyhow::Result<()> {
    // Setup - create VM with storage backend
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set auth context
    let auth_context = AuthContext::new("test-user");
    vm.set_auth_context(auth_context);
    
    // Define proposal parameters
    let namespace = "test-namespace";
    let proposal_id = "test-proposal-threshold-fail";
    let proposal_title = "Test Proposal (Threshold Fail)";
    let proposal_description = "This is a test proposal that should fail threshold";
    let min_quorum = 0.5; // 50% quorum
    let vote_threshold = 0.6; // 60% approval threshold
    let min_deliberation_period = 3600; // 1 hour
    
    // Create proposal
    create_proposal(
        &mut vm,
        namespace,
        proposal_id,
        proposal_title,
        proposal_description,
        min_quorum,
        vote_threshold,
        min_deliberation_period,
    )?;
    
    // Open for feedback
    let result = open_for_feedback(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Start voting phase
    let result = start_voting(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Cast votes - enough for quorum (70%) but not enough yes votes to pass threshold (only 40% yes)
    cast_votes(
        &mut vm,
        namespace,
        proposal_id,
        &[
            ("voter1", 0.4, true),   // 40% yes
            ("voter2", 0.3, false),  // 30% no
        ],
    )?;
    
    // Verify votes were correctly recorded
    let votes = get_proposal_votes(&vm, namespace, proposal_id)?;
    assert_eq!(votes.len(), 2);
    
    // Calculate vote totals
    let total_weight: f64 = votes.iter().map(|(_, weight, _)| *weight).sum();
    let yes_weight: f64 = votes
        .iter()
        .filter(|(_, _, vote)| *vote)
        .map(|(_, weight, _)| *weight)
        .sum();
    
    // Verify quorum met but threshold not met
    assert!(total_weight >= min_quorum, "Should have reached quorum");
    assert!(yes_weight / total_weight < vote_threshold, "Should not have passed threshold");
    
    // Try to execute proposal - should fail due to not meeting threshold
    let result = execute_proposal(&mut vm, namespace, proposal_id)?;
    assert!(!result.get("success").unwrap().as_bool().unwrap());
    
    // Check proposal is still in voting state
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "VotingPhase");
    
    Ok(())
}

#[test]
fn test_proposal_execution_with_dsl_file() -> anyhow::Result<()> {
    // Setup - create VM with storage backend
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set auth context
    let auth_context = AuthContext::new("test-user");
    vm.set_auth_context(auth_context.clone());
    
    // Read the DSL file
    let dsl_path = "demo/governance/proposal_lifecycle_retry.dsl";
    let dsl_content = fs::read_to_string(dsl_path)
        .expect("Failed to read DSL file");
    
    // Parse the DSL content
    let (ops, config_opt) = parse_dsl(&dsl_content)
        .expect("Failed to parse DSL content");
    
    // Extract governance parameters from the DSL
    let lifecycle_config = config_opt.expect("Missing lifecycle config");
    let min_quorum = lifecycle_config.governance.quorum.unwrap_or(0.5);
    let vote_threshold = lifecycle_config.governance.threshold.unwrap_or(0.5);
    let min_deliberation_period = lifecycle_config.governance.min_deliberation.unwrap_or(3600);
    
    // Define proposal parameters
    let namespace = "test-namespace";
    let proposal_id = "test-proposal-dsl";
    let proposal_title = lifecycle_config.title.unwrap_or_else(|| "Default Title".to_string());
    let proposal_description = lifecycle_config.description.unwrap_or_else(|| "Default Description".to_string());
    
    // Create proposal using parameters from the DSL
    create_proposal(
        &mut vm,
        namespace,
        proposal_id,
        &proposal_title,
        &proposal_description,
        min_quorum,
        vote_threshold,
        min_deliberation_period,
    )?;
    
    // Store the DSL operations with the proposal for execution
    let mut storage = vm.get_storage_backend_mut().unwrap();
    let dsl_key = format!("governance/proposal/{}/dsl", proposal_id);
    let ops_json = serde_json::to_vec(&ops).expect("Failed to serialize operations");
    storage.set(Some(&auth_context), namespace, &dsl_key, ops_json)?;
    
    // Check if proposal exists and has the correct parameters
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("id").unwrap().as_str().unwrap(), proposal_id);
    assert_eq!(proposal.get("title").unwrap().as_str().unwrap(), proposal_title);
    assert_eq!(proposal.get("description").unwrap().as_str().unwrap(), proposal_description);
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "DraftPhase");
    assert_eq!(proposal.get("quorum").unwrap().as_f64().unwrap(), min_quorum);
    assert_eq!(proposal.get("threshold").unwrap().as_f64().unwrap(), vote_threshold);
    
    // Move proposal through the lifecycle
    open_for_feedback(&mut vm, namespace, proposal_id)?;
    start_voting(&mut vm, namespace, proposal_id)?;
    
    // Cast votes - enough for quorum and threshold
    cast_votes(
        &mut vm,
        namespace,
        proposal_id,
        &[
            ("voter1", 0.6, true),  // 60% yes
            ("voter2", 0.1, false), // 10% no
        ],
    )?;
    
    // Verify votes were correctly recorded
    let votes = get_proposal_votes(&vm, namespace, proposal_id)?;
    assert_eq!(votes.len(), 2);
    
    // Calculate vote totals
    let total_weight: f64 = votes.iter().map(|(_, weight, _)| *weight).sum();
    let yes_weight: f64 = votes
        .iter()
        .filter(|(_, _, vote)| *vote)
        .map(|(_, weight, _)| *weight)
        .sum();
    
    // Verify quorum and threshold met
    assert!(total_weight >= min_quorum, "Should have reached quorum");
    assert!(yes_weight / total_weight >= vote_threshold, "Should have passed threshold");
    
    // First attempt should fail because the missing key doesn't exist
    vm.set_missing_key_behavior(icn_covm::vm::MissingKeyBehavior::Error);
    let result = execute_proposal(&mut vm, namespace, proposal_id)?;
    assert!(!result.get("success").unwrap().as_bool().unwrap());
    
    // Verify retry count has been incremented
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    let retries = proposal.get("execution_retries").unwrap().as_u64().unwrap();
    assert_eq!(retries, 1, "Should have tracked one retry attempt");
    
    // Create the missing key to make the next attempt succeed
    let mut storage = vm.get_storage_backend_mut().unwrap();
    storage.set(
        Some(&auth_context),
        namespace,
        "missing/key",
        b"success-value".to_vec(),
    )?;
    
    // Second attempt should succeed now that the key exists
    let result = execute_proposal(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Verify that the proposal state has changed to ExecutionPhase
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "ExecutionPhase");
    
    // Verify the result key has been set
    let storage = vm.get_storage_backend().unwrap();
    let result_bytes = storage.get(Some(&auth_context), namespace, "result-key")?;
    let result_value = String::from_utf8(result_bytes).expect("Valid UTF-8");
    assert_eq!(result_value, "success-value");
    
    Ok(())
}

#[test]
fn test_proposal_execution_with_retry_integration() -> anyhow::Result<()> {
    // Setup - create VM with storage backend
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set auth context
    let auth_context = AuthContext::new("test-user");
    vm.set_auth_context(auth_context.clone());
    
    // Define proposal parameters
    let namespace = "test-namespace";
    let proposal_id = "test-proposal-retry-integration";
    let proposal_title = "Test Proposal Retry Integration";
    let proposal_description = "This proposal tests integration with retry functionality";
    let min_quorum = 0.5; // 50% quorum
    let vote_threshold = 0.5; // 50% approval threshold
    let min_deliberation_period = 3600; // 1 hour
    
    // Create proposal
    create_proposal(
        &mut vm,
        namespace,
        proposal_id,
        proposal_title,
        proposal_description,
        min_quorum,
        vote_threshold,
        min_deliberation_period,
    )?;
    
    // Read the DSL file for operations
    let dsl_path = "demo/governance/proposal_lifecycle_retry.dsl";
    let dsl_content = fs::read_to_string(dsl_path)
        .expect("Failed to read DSL file");
    
    // Parse the DSL content for operations
    let (ops, _) = parse_dsl(&dsl_content)
        .expect("Failed to parse DSL content");
    
    // Store the operations with the proposal
    let mut storage = vm.get_storage_backend_mut().unwrap();
    let dsl_key = format!("governance/proposal/{}/dsl", proposal_id);
    let ops_json = serde_json::to_vec(&ops).expect("Failed to serialize operations");
    storage.set(Some(&auth_context), namespace, &dsl_key, ops_json)?;
    
    // Open for feedback
    let result = open_for_feedback(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Start voting phase
    let result = start_voting(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Cast votes - enough for quorum and threshold
    cast_votes(
        &mut vm,
        namespace,
        proposal_id,
        &[
            ("voter1", 0.6, true),  // 60% yes
            ("voter2", 0.1, false), // 10% no
        ],
    )?;
    
    // Set VM to fail when keys are not found
    vm.set_missing_key_behavior(icn_covm::vm::MissingKeyBehavior::Error);
    
    // First execution attempt should fail because missing/key doesn't exist
    let result = execute_proposal(&mut vm, namespace, proposal_id)?;
    assert!(!result.get("success").unwrap().as_bool().unwrap());
    
    // Verify retry count has been incremented
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    let retries = proposal.get("execution_retries").unwrap().as_u64().unwrap();
    assert_eq!(retries, 1, "Should have tracked one retry attempt");
    
    // Create the missing key to satisfy the operation
    let mut storage = vm.get_storage_backend_mut().unwrap();
    storage.set(
        Some(&auth_context),
        namespace,
        "missing/key",
        b"success-value".to_vec(),
    )?;
    
    // Second execution attempt should succeed now that the key exists
    let result = execute_proposal(&mut vm, namespace, proposal_id)?;
    assert!(result.get("success").unwrap().as_bool().unwrap());
    
    // Verify proposal state changed to ExecutionPhase
    let proposal = get_proposal(&vm, namespace, proposal_id)?;
    assert_eq!(proposal.get("state").unwrap().as_str().unwrap(), "ExecutionPhase");
    
    // Verify the result key was set correctly
    let storage = vm.get_storage_backend().unwrap();
    let result_bytes = storage.get(Some(&auth_context), namespace, "result-key")?;
    let result_value = String::from_utf8(result_bytes).expect("Valid UTF-8");
    assert_eq!(result_value, "success-value");
    
    Ok(())
}

// New helper that doesn't capture auth from the VM but takes it as parameter
fn cast_votes_with_auth(
    vm: &mut VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
    votes: &[(&str, f64, bool)],
    auth_context: &AuthContext
) -> StorageResult<()> {
    let mut storage = vm.get_storage_backend_mut().unwrap();
    cast_votes_internal(&mut storage, namespace, proposal_id, votes, auth_context)
}

// New helper that doesn't capture auth from the VM but takes it as parameter
fn get_proposal_votes_with_auth(
    vm: &VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
    auth_context: &AuthContext
) -> StorageResult<Vec<(String, f64, bool)>> {
    let storage = vm.get_storage_backend().unwrap();
    get_proposal_votes_internal(storage, namespace, proposal_id, auth_context)
}

// Casts votes for a proposal
fn cast_votes(
    vm: &mut VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
    votes: &[(&str, f64, bool)]
) -> StorageResult<()> {
    // Get auth_context before getting mutable storage to avoid borrow checker issues
    let auth_context = vm.get_auth_context().expect("Auth context should be set").clone();
    let mut storage = vm.get_storage_backend_mut().unwrap();
    cast_votes_internal(&mut storage, namespace, proposal_id, votes, &auth_context)
}

// Internal function to cast votes
fn cast_votes_internal(
    storage: &mut InMemoryStorage,
    namespace: &str,
    proposal_id: &str,
    votes: &[(&str, f64, bool)],
    auth_context: &AuthContext
) -> StorageResult<()> {
    for (voter, weight, choice) in votes {
        let vote_key = format!("governance/proposal/{}/votes/{}", proposal_id, voter);
        let vote_data = json!({
            "weight": weight,
            "choice": choice
        });
        let vote_bytes = serde_json::to_vec(&vote_data).unwrap();
        storage.set(Some(auth_context), namespace, &vote_key, vote_bytes)?;
    }
    Ok(())
}

// Gets votes for a proposal
fn get_proposal_votes(
    vm: &VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
) -> StorageResult<Vec<(String, f64, bool)>> {
    let auth_context = vm.get_auth_context().expect("Auth context should be set").clone();
    let storage = vm.get_storage_backend().unwrap();
    get_proposal_votes_internal(storage, namespace, proposal_id, &auth_context)
}

// Internal function to get proposal votes
fn get_proposal_votes_internal(
    storage: &InMemoryStorage,
    namespace: &str,
    proposal_id: &str,
    auth_context: &AuthContext
) -> StorageResult<Vec<(String, f64, bool)>> {
    let vote_prefix = format!("governance/proposal/{}/votes/", proposal_id);
    let keys = storage.list(Some(auth_context), namespace, &vote_prefix)?;
    let mut votes = Vec::new();
    
    for key in keys {
        if key.starts_with(&vote_prefix) {
            let voter = key.strip_prefix(&vote_prefix).unwrap().to_string();
            let vote_bytes = storage.get(Some(auth_context), namespace, &key)?;
            let vote_data: Value = serde_json::from_slice(&vote_bytes)
                .expect("Should deserialize vote data");
            
            let weight = vote_data["weight"].as_f64().unwrap_or(0.0);
            let choice = vote_data["choice"].as_bool().unwrap_or(false);
            
            votes.push((voter, weight, choice));
        }
    }
    
    Ok(votes)
}

// Gets execution results for a proposal
fn get_execution_results(
    vm: &VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
) -> StorageResult<Vec<Value>> {
    let auth_context = vm.get_auth_context().expect("Auth context should be set").clone();
    let storage = vm.get_storage_backend().unwrap();
    get_execution_results_internal(storage, namespace, proposal_id, &auth_context)
}

// Internal function to get execution results
fn get_execution_results_internal(
    storage: &InMemoryStorage,
    namespace: &str,
    proposal_id: &str,
    auth_context: &AuthContext
) -> StorageResult<Vec<Value>> {
    let execution_key = format!("governance/proposal/{}/execution", proposal_id);
    
    match storage.get(Some(auth_context), namespace, &execution_key) {
        Ok(bytes) => {
            let results: Vec<Value> = serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Vec::new());
            Ok(results)
        },
        Err(StorageError::NotFound { .. }) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

// Execute the proposal logic
fn execute_proposal_logic(
    vm: &mut VM<InMemoryStorage>,
    ops: &[icn_covm::vm::types::Op]
) -> Result<Value, String> {
    // Execute the operations
    vm.execute(ops).map_err(|e| format!("Execution failed: {}", e))?;
    
    // Get the result from the stack (if any)
    let result = match vm.top() {
        Some(val) => json!(val),
        None => json!(null),
    };
    
    Ok(result)
}

// Helper trait extension for setting JSON values directly
trait StorageJsonExtension {
    fn set_json<T: serde::Serialize>(
        &mut self, 
        auth: Option<&AuthContext>, 
        namespace: &str, 
        key: &str, 
        value: &T
    ) -> StorageResult<()>;
}

impl StorageJsonExtension for InMemoryStorage {
    fn set_json<T: serde::Serialize>(
        &mut self, 
        auth: Option<&AuthContext>, 
        namespace: &str, 
        key: &str, 
        value: &T
    ) -> StorageResult<()> {
        let bytes = serde_json::to_vec(value).expect("Failed to serialize value");
        self.set(auth, namespace, key, bytes)
    }
}

// Create proposal
fn create_proposal(
    vm: &mut VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
    title: &str,
    description: &str,
    min_quorum: f64,
    vote_threshold: f64,
    min_deliberation_period: i64,
) -> anyhow::Result<Value> {
    let auth_context = vm.get_auth_context().expect("Auth context should be set").clone();
    let mut storage = vm.get_storage_backend_mut().unwrap();
    
    let proposal = ProposalLifecycle::new(
        proposal_id.to_string(),
        auth_context.identity.clone(),
        title,
        description,
        min_quorum,
        vote_threshold,
        min_deliberation_period,
    );
    
    let proposal_key = format!("governance/proposal/{}", proposal_id);
    let proposal_json = serde_json::to_value(&proposal).unwrap();
    
    storage.set_json(Some(&auth_context), namespace, &proposal_key, &proposal_json)?;
    
    Ok(json!({
        "success": true,
        "proposal": proposal_json
    }))
}

// Open proposal for feedback
fn open_for_feedback(
    vm: &mut VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
) -> anyhow::Result<Value> {
    let auth_context = vm.get_auth_context().expect("Auth context should be set").clone();
    let mut storage = vm.get_storage_backend_mut().unwrap();
    
    let proposal_key = format!("governance/proposal/{}", proposal_id);
    let proposal_bytes = storage.get(Some(&auth_context), namespace, &proposal_key)?;
    let mut proposal: ProposalLifecycle = serde_json::from_slice(&proposal_bytes)
        .expect("Should deserialize proposal data");
    
    match proposal.open_for_feedback() {
        Ok(_) => {
            let proposal_json = serde_json::to_value(&proposal).unwrap();
            storage.set_json(Some(&auth_context), namespace, &proposal_key, &proposal_json)?;
            
            Ok(json!({
                "success": true,
                "proposal": proposal_json
            }))
        },
        Err(e) => {
            Ok(json!({
                "success": false,
                "error": e.to_string()
            }))
        }
    }
}

// Start voting phase
fn start_voting(
    vm: &mut VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
) -> anyhow::Result<Value> {
    let auth_context = vm.get_auth_context().expect("Auth context should be set").clone();
    let mut storage = vm.get_storage_backend_mut().unwrap();
    
    let proposal_key = format!("governance/proposal/{}", proposal_id);
    let proposal_bytes = storage.get(Some(&auth_context), namespace, &proposal_key)?;
    let mut proposal: ProposalLifecycle = serde_json::from_slice(&proposal_bytes)
        .expect("Should deserialize proposal data");
    
    match proposal.start_voting() {
        Ok(_) => {
            let proposal_json = serde_json::to_value(&proposal).unwrap();
            storage.set_json(Some(&auth_context), namespace, &proposal_key, &proposal_json)?;
            
            Ok(json!({
                "success": true,
                "proposal": proposal_json
            }))
        },
        Err(e) => {
            Ok(json!({
                "success": false,
                "error": e.to_string()
            }))
        }
    }
}

// Execute proposal
fn execute_proposal(
    vm: &mut VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
) -> anyhow::Result<Value> {
    execute_proposal_with_params(vm, namespace, proposal_id, "")
}

// Execute proposal with retry parameters
fn execute_proposal_with_params(
    vm: &mut VM<InMemoryStorage>,
    namespace: &str,
    proposal_id: &str,
    retry_params: &str,
) -> anyhow::Result<Value> {
    let auth_context = vm.get_auth_context().expect("Auth context should be set").clone();
    let mut storage = vm.get_storage_backend_mut().unwrap();
    
    let proposal_key = format!("governance/proposal/{}", proposal_id);
    let proposal_bytes = storage.get(Some(&auth_context), namespace, &proposal_key)?;
    let mut proposal: ProposalLifecycle = serde_json::from_slice(&proposal_bytes)
        .expect("Should deserialize proposal data");
    
    // Check if votes meet quorum and threshold
    let votes = get_proposal_votes_internal(&storage, namespace, proposal_id, &auth_context)?;
    let total_weight: f64 = votes.iter().map(|(_, weight, _)| *weight).sum();
    let yes_weight: f64 = votes
        .iter()
        .filter(|(_, _, vote)| *vote)
        .map(|(_, weight, _)| *weight)
        .sum();
    
    if total_weight < proposal.quorum {
        return Ok(json!({
            "success": false,
            "error": format!("Quorum not met: {} < {}", total_weight, proposal.quorum)
        }));
    }
    
    if yes_weight / total_weight < proposal.threshold {
        return Ok(json!({
            "success": false,
            "error": format!("Threshold not met: {} < {}", yes_weight / total_weight, proposal.threshold)
        }));
    }
    
    // Check if there are DSL operations associated with this proposal
    let dsl_key = format!("governance/proposal/{}/dsl", proposal_id);
    let ops = match storage.get(Some(&auth_context), namespace, &dsl_key) {
        Ok(bytes) => {
            // We have DSL operations stored
            let ops: Vec<icn_covm::vm::types::Op> = serde_json::from_slice(&bytes)
                .expect("Should deserialize operations");
            Some(ops)
        },
        Err(StorageError::NotFound { .. }) => None,
        Err(e) => return Err(e.into()),
    };
    
    // If retry params are provided, simulate retry behavior
    if !retry_params.is_empty() {
        let retry_params: Value = serde_json::from_str(retry_params)
            .unwrap_or_else(|_| json!({}));
        
        let fail_on_retry = retry_params.get("failOnRetry")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        
        let max_retries = retry_params.get("maxRetries")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        
        // Get current retry count
        let retries = proposal.execution_retries.unwrap_or(0);
        
        // Increment retry count
        proposal.execution_retries = Some(retries + 1);
        
        // Update proposal with new retry count
        let proposal_json = serde_json::to_value(&proposal).unwrap();
        storage.set_json(Some(&auth_context), namespace, &proposal_key, &proposal_json)?;
        
        // Check if max retries reached
        if fail_on_retry {
            if retries >= max_retries {
                return Ok(json!({
                    "success": false,
                    "error": format!("Execution failed after max retries ({})", max_retries)
                }));
            } else {
                return Ok(json!({
                    "success": false,
                    "error": format!("Execution failed on retry {}", retries + 1)
                }));
            }
        }
    }
    
    // Execute any DSL operations if available
    if let Some(ops) = ops {
        match execute_proposal_logic(vm, &ops) {
            Ok(_) => {
                // Success - update proposal state
                match proposal.execute() {
                    Ok(_) => {
                        let proposal_json = serde_json::to_value(&proposal).unwrap();
                        storage.set_json(Some(&auth_context), namespace, &proposal_key, &proposal_json)?;
                        
                        Ok(json!({
                            "success": true,
                            "proposal": proposal_json
                        }))
                    },
                    Err(e) => {
                        Ok(json!({
                            "success": false,
                            "error": e.to_string()
                        }))
                    }
                }
            },
            Err(e) => {
                // Execution failed - keep the retry count but don't transition state
                let proposal_json = serde_json::to_value(&proposal).unwrap();
                storage.set_json(Some(&auth_context), namespace, &proposal_key, &proposal_json)?;
                
                Ok(json!({
                    "success": false,
                    "error": e
                }))
            }
        }
    } else {
        // No DSL operations, just transition state
        match proposal.execute() {
            Ok(_) => {
                let proposal_json = serde_json::to_value(&proposal).unwrap();
                storage.set_json(Some(&auth_context), namespace, &proposal_key, &proposal_json)?;
                
                Ok(json!({
                    "success": true,
                    "proposal": proposal_json
                }))
            },
            Err(e) => {
                Ok(json!({
                    "success": false,
                    "error": e.to_string()
                }))
            }
        }
    }
}

// Get proposal details
fn get_proposal(
    vm: &VM<InMemoryStorage>, 
    namespace: &str,
    proposal_id: &str
) -> StorageResult<Value> {
    let auth_context = vm.get_auth_context().expect("Auth context should be set").clone();
    let storage = vm.get_storage_backend().unwrap();
    
    let proposal_key = format!("governance/proposal/{}", proposal_id);
    let proposal_bytes = storage.get(Some(&auth_context), namespace, &proposal_key)?;
    let proposal: Value = serde_json::from_slice(&proposal_bytes)
        .expect("Should deserialize proposal data");
    
    Ok(proposal)
}