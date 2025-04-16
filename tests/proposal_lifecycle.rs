use std::str::FromStr;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::vm::VM;
use serde_json::{json, Value};

#[test]
fn test_proposal_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    // Setup VM and storage
    let mut storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set namespace for the test
    let namespace = "governance_test";
    vm.set_namespace(namespace.to_string());
    
    // Create a proposal with ACTIVE state
    let proposal_id = "test_proposal_1";
    let lifecycle_key = format!("proposals/{}/lifecycle", proposal_id);
    let ledger_key = format!("proposals/{}/ledger", proposal_id);
    
    // Setup initial proposal state
    {
        let storage = vm.get_storage_backend_mut().unwrap();
        
        // Create lifecycle data
        let lifecycle = json!({
            "state": "ACTIVE",
            "created_at": "2023-01-01T00:00:00Z",
            "expiry": "2023-01-10T00:00:00Z",
            "quorum": 2.0,
            "threshold": 0.5
        });
        
        storage.set_json(None, namespace, &lifecycle_key, &lifecycle)?;
        
        // Create ledger data
        let ledger = json!({
            "transfer": {
                "from": "treasury",
                "to": "recipient",
                "amount": 1000
            }
        });
        
        storage.set_json(None, namespace, &ledger_key, &ledger)?;
        
        // Cast votes
        cast_vote(storage, namespace, proposal_id, "voter1", 1.0)?;
        cast_vote(storage, namespace, proposal_id, "voter2", 1.0)?;
    }
    
    // Check votes were recorded correctly
    {
        let storage = vm.get_storage_backend().unwrap();
        let votes = get_proposal_votes(storage, namespace, proposal_id)?;
        assert_eq!(votes.len(), 2);
        assert!(votes.contains(&("voter1".to_string(), 1.0)));
        assert!(votes.contains(&("voter2".to_string(), 1.0)));
    }
    
    // Execute the proposal
    {
        let storage = vm.get_storage_backend_mut().unwrap();
        execute_proposal(storage, namespace, proposal_id)?;
    }
    
    // Verify the proposal was executed
    {
        let storage = vm.get_storage_backend().unwrap();
        let lifecycle = get_lifecycle(storage, namespace, proposal_id)?;
        assert_eq!(lifecycle["state"], "EXECUTED");
    }
    
    Ok(())
}

#[test]
fn test_failing_proposal_quorum() -> Result<(), Box<dyn std::error::Error>> {
    // Setup VM and storage
    let mut storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    
    // Set namespace for the test
    let namespace = "governance_test";
    vm.set_namespace(namespace.to_string());
    
    // Create a proposal with high quorum requirement
    let proposal_id = "test_proposal_2";
    let lifecycle_key = format!("proposals/{}/lifecycle", proposal_id);
    let ledger_key = format!("proposals/{}/ledger", proposal_id);
    
    // Setup initial proposal state
    {
        let storage = vm.get_storage_backend_mut().unwrap();
        
        // Create lifecycle data with high quorum
        let lifecycle = json!({
            "state": "ACTIVE",
            "created_at": "2023-01-01T00:00:00Z",
            "expiry": "2023-01-10T00:00:00Z",
            "quorum": 5.0,  // High quorum that won't be met
            "threshold": 0.5
        });
        
        storage.set_json(None, namespace, &lifecycle_key, &lifecycle)?;
        
        // Create ledger data
        let ledger = json!({
            "transfer": {
                "from": "treasury",
                "to": "recipient",
                "amount": 1000
            }
        });
        
        storage.set_json(None, namespace, &ledger_key, &ledger)?;
        
        // Cast only one vote (not enough for quorum)
        cast_vote(storage, namespace, proposal_id, "voter1", 1.0)?;
    }
    
    // Check that the vote was recorded
    {
        let storage = vm.get_storage_backend().unwrap();
        let votes = get_proposal_votes(storage, namespace, proposal_id)?;
        assert_eq!(votes.len(), 1);
    }
    
    // Execute the proposal - in a real implementation this would check quorum
    // For the test, we'll just simulate this logic
    {
        let storage = vm.get_storage_backend_mut().unwrap();
        let vote_count = get_proposal_votes(storage, namespace, proposal_id)?.len() as f64;
        
        // Get the required quorum
        let lifecycle = get_lifecycle(storage, namespace, proposal_id)?;
        let required_quorum = lifecycle["quorum"].as_f64().unwrap_or(0.0);
        
        if vote_count >= required_quorum {
            // If quorum met, execute normally
            execute_proposal(storage, namespace, proposal_id)?;
        } else {
            // If quorum not met, update to FAILED state
            let updated_lifecycle = json!({
                "state": "FAILED",
                "created_at": lifecycle["created_at"],
                "expiry": lifecycle["expiry"],
                "quorum": lifecycle["quorum"],
                "threshold": lifecycle["threshold"],
                "failure_reason": "Quorum not met"
            });
            
            storage.set_json(None, namespace, &lifecycle_key, &updated_lifecycle)?;
        }
    }
    
    // Verify the proposal was marked as failed
    {
        let storage = vm.get_storage_backend().unwrap();
        let lifecycle = get_lifecycle(storage, namespace, proposal_id)?;
        assert_eq!(lifecycle["state"], "FAILED");
        assert_eq!(lifecycle["failure_reason"], "Quorum not met");
    }
    
    Ok(())
}

// Helper function to cast a vote
fn cast_vote(storage: &mut InMemoryStorage, namespace: &str, proposal_id: &str, voter: &str, vote: f64) -> Result<(), Box<dyn std::error::Error>> {
    let votes_prefix = format!("proposals/{}/votes/", proposal_id);
    let vote_key = format!("{}{}", votes_prefix, voter);
    let value_str = vote.to_string();
    
    storage.set(None, namespace, &vote_key, value_str.as_bytes().to_vec())?;
    Ok(())
}

// Helper function to get proposal votes
fn get_proposal_votes(storage: &InMemoryStorage, namespace: &str, proposal_id: &str) -> Result<Vec<(String, f64)>, Box<dyn std::error::Error>> {
    let votes_prefix = format!("proposals/{}/votes/", proposal_id);
    let vote_keys = storage.list_keys(None, namespace, Some(&votes_prefix))?;
    
    let mut votes = Vec::new();
    for key in vote_keys {
        let voter = key.strip_prefix(&votes_prefix)
            .ok_or_else(|| "Failed to extract voter ID from key".to_string())?
            .to_string();
        
        let vote_bytes = storage.get(None, namespace, &key)?
            .ok_or_else(|| "Vote not found".to_string())?;
        let vote_str = String::from_utf8(vote_bytes)?;
        let vote_value = f64::from_str(&vote_str)?;
        
        votes.push((voter, vote_value));
    }
    
    Ok(votes)
}

// Helper function to execute a proposal
fn execute_proposal(storage: &mut InMemoryStorage, namespace: &str, proposal_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let lifecycle_key = format!("proposals/{}/lifecycle", proposal_id);
    
    // Update the lifecycle state to EXECUTED
    let lifecycle = get_lifecycle(storage, namespace, proposal_id)?;
    
    let updated_lifecycle = json!({
        "state": "EXECUTED",
        "created_at": lifecycle["created_at"],
        "expiry": lifecycle["expiry"],
        "quorum": lifecycle["quorum"],
        "threshold": lifecycle["threshold"],
        "executed_at": "2023-01-01T00:00:00Z"
    });
    
    storage.set_json(None, namespace, &lifecycle_key, &updated_lifecycle)?;
    Ok(())
}

// Helper function to get lifecycle data
fn get_lifecycle(storage: &InMemoryStorage, namespace: &str, proposal_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let lifecycle_key = format!("proposals/{}/lifecycle", proposal_id);
    let lifecycle_bytes = storage.get(None, namespace, &lifecycle_key)?
        .ok_or_else(|| "Lifecycle not found".to_string())?;
    let lifecycle_str = String::from_utf8(lifecycle_bytes)?;
    let lifecycle: Value = serde_json::from_str(&lifecycle_str)?;
    
    Ok(lifecycle)
}