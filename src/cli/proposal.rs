use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use chrono::Utc;

use crate::VM;
use crate::Storage;
use crate::StorageExtensions;
use crate::storage::auth::AuthContext;
use crate::governance::ProposalComment;
use crate::governance::ExecutionStatus;

/// Creates the proposal command structure for the CLI
pub fn proposal_command() -> clap::Command {
    clap::Command::new("proposal")
        .about("Manage governance proposals")
        .subcommand(
            clap::Command::new("execute")
                .about("Execute an approved proposal")
                .arg(
                    clap::Arg::new("id")
                        .required(true)
                        .help("Proposal ID to execute")
                )
        )
        .subcommand(
            clap::Command::new("retry-history")
                .about("View execution retry history for a proposal")
                .arg(
                    clap::Arg::new("id")
                        .required(true)
                        .help("Proposal ID to check")
                )
        )
}

/// Handle the execute command for a proposal
pub fn handle_execute_command<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
    auth_context: &AuthContext,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    println!("Executing proposal {}...", proposal_id);
    
    // Load the proposal lifecycle
    let mut proposal = load_proposal(vm, proposal_id)?;
    
    // Check if the proposal is in the right state for execution
    // This will depend on your specific workflow
    // For now, we'll just call the transition_to_executed method
    
    match proposal.transition_to_executed(vm, Some(auth_context)) {
        Ok(executed) => {
            if executed {
                println!("✅ Proposal executed successfully.");
                
                // Get execution status
                if let Some(status) = &proposal.execution_status {
                    match status {
                        ExecutionStatus::Success => {
                            println!("Execution succeeded.");
                        },
                        ExecutionStatus::Failure(reason) => {
                            println!("⚠️ Execution failed: {}", reason);
                        }
                    }
                }
                
                Ok(())
            } else {
                Err("Failed to execute proposal - it may not meet voting requirements.".into())
            }
        },
        Err(e) => {
            println!("❌ Execution failed: {}", e);
            Err(format!("Failed to execute proposal: {}", e).into())
        }
    }
}

/// Handle the retry-history command to view execution retry history for a proposal
pub fn handle_retry_history_command<S>(
    vm: &mut VM<S>,
    proposal_id: &str,
) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm.storage_backend.as_ref().ok_or("Storage backend not configured")?;
    
    // Get the proposal to verify it exists
    let proposal_key = format!("proposals/{}/metadata", proposal_id);
    match storage.get(None, "governance", &proposal_key) {
        Ok(_) => {
            // Proposal exists, get retry history
            match storage.get_proposal_retry_history(proposal_id) {
                Ok(records) => {
                    if records.is_empty() {
                        println!("No retry history found for proposal {}", proposal_id);
                        return Ok(());
                    }
                    
                    println!("Retry history for proposal {}:", proposal_id);
                    println!("{}", "-".repeat(80));
                    println!("{:<24} | {:<15} | {:<8} | {:<8} | {}", 
                             "TIMESTAMP", "USER", "STATUS", "ATTEMPT", "REASON");
                    println!("{}", "-".repeat(80));
                    
                    for record in records {
                        let retry_count = record.retry_count.map_or("N/A".to_string(), |c| c.to_string());
                        let reason = record.reason.unwrap_or_else(|| "-".to_string());
                        
                        println!("{:<24} | {:<15} | {:<8} | {:<8} | {}", 
                                 record.timestamp, 
                                 record.user, 
                                 record.status, 
                                 retry_count,
                                 reason);
                    }
                    println!("{}", "-".repeat(80));
                    
                    // Additional information about cooldown
                    if let Some(record) = records.first() {
                        if record.status == "success" {
                            println!("✅ Latest retry was successful.");
                        } else {
                            use crate::utils::time;
                            use crate::governance::proposal_lifecycle::COOLDOWN_DURATION;
                            
                            // Parse the timestamp from the record
                            if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(&record.timestamp) {
                                let utc_timestamp = timestamp.with_timezone(&chrono::Utc);
                                let remaining = time::get_cooldown_remaining(&utc_timestamp.to_string(), COOLDOWN_DURATION);
                                
                                if remaining.num_seconds() > 0 {
                                    println!("⏳ Cooldown: Wait {} before next retry allowed", 
                                             time::format_duration(remaining));
                                } else {
                                    println!("✅ Cooldown: Ready for next retry");
                                }
                            }
                        }
                    }
                    
                    Ok(())
                },
                Err(e) => Err(format!("Failed to get retry history: {}", e).into()),
            }
        },
        Err(e) => Err(format!("Proposal {} not found: {}", proposal_id, e).into()),
    }
}

/// Loads a proposal from storage by ID
fn load_proposal<S>(vm: &mut VM<S>, proposal_id: &str) -> Result<crate::governance::proposal_lifecycle::ProposalLifecycle, Box<dyn Error>>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm.storage_backend.as_ref().ok_or("Storage backend not configured")?;
    let proposal_key = format!("proposals/{}/metadata", proposal_id);
    
    let data = storage.get(None, "governance", &proposal_key)
        .map_err(|e| format!("Failed to load proposal {}: {}", proposal_id, e))?;
    
    let proposal: crate::governance::proposal_lifecycle::ProposalLifecycle = serde_json::from_slice(&data)
        .map_err(|e| format!("Failed to parse proposal {}: {}", proposal_id, e))?;
    
    Ok(proposal)
}

// Helper functions for tests
#[cfg(test)]
fn setup_test_vm() -> VM<impl Storage> {
    use crate::storage::implementations::in_memory::InMemoryStorage;
    let storage = InMemoryStorage::new();
    VM::new_with_storage(storage)
}

#[cfg(test)]
fn setup_test_auth() -> AuthContext {
    AuthContext {
        current_identity_did: "did:key:test".to_string(),
        current_identity_alias: Some("TestUser".to_string()),
        roles: vec!["admin".to_string()],
    }
}

#[cfg(test)]
fn create_test_proposal<S: Storage>(vm: &mut VM<S>, proposal_id: &str) -> Result<(), Box<dyn Error>> {
    let proposal_key = format!("proposals/{}/metadata", proposal_id);
    let proposal = crate::governance::proposal_lifecycle::ProposalLifecycle::new(
        proposal_id.to_string(),
        "Test Proposal".to_string(),
        "Test description".to_string(),
        "did:key:test".to_string(),
        55, // quorum percentage
        60, // threshold percentage
    );
    
    let proposal_data = serde_json::to_vec(&proposal)?;
    
    vm.storage_backend.as_mut().unwrap().set(
        None,
        "governance",
        &proposal_key,
        proposal_data,
    )?;
    
    Ok(())
}

#[test]
fn test_comment_reactions() -> Result<(), Box<dyn Error>> {
    let mut vm = setup_test_vm();
    let auth = setup_test_auth();
    let proposal_id = "test-proposal";
    let comment_id = "comment1";

    // Create test proposal
    create_test_proposal(&mut vm, proposal_id)?;

    // Create a comment with reactions
    let comment_key = format!("comments/{}/{}", proposal_id, comment_id);

    let mut reactions = HashMap::new();
    reactions.insert("👍".to_string(), 1);

    let comment = ProposalComment {
        id: comment_id.to_string(),
        author: auth.current_identity_did.clone(),
        timestamp: Utc::now(),
        content: "This is a test comment".to_string(),
        reply_to: None,
        tags: Vec::new(),
        reactions,
    };

    let comment_data = serde_json::to_vec(&comment)?;

    vm.storage_backend.as_mut().unwrap().set(
        Some(&auth),
        "comments",
        &comment_key,
        comment_data,
    )?;

    // Retrieve the comment
    let retrieved_data =
        vm.storage_backend
            .as_ref()
            .unwrap()
            .get(Some(&auth), "comments", &comment_key)?;

    let retrieved_comment: ProposalComment = serde_json::from_slice(&retrieved_data)?;

    // Verify reactions are present
    assert_eq!(retrieved_comment.reactions.len(), 1);
    assert_eq!(retrieved_comment.reactions.get("👍"), Some(&1));

    Ok(())
}

// Test retrieving proposal retry history
#[test]
fn test_retry_history() -> Result<(), Box<dyn Error>> {
    let mut vm = setup_test_vm();
    let proposal_id = "test-proposal-retry";
    
    // Create test proposal
    create_test_proposal(&mut vm, proposal_id)?;
    
    // Manually add some execution logs with retry information
    let logs_key = format!("proposals/{}/execution_logs", proposal_id);
    let retry_logs = vec![
        "[2023-06-15T14:30:00Z] RETRY by user:admin | status: failed | retry_count: 1 | reason: VM execution error",
        "[2023-06-15T15:45:00Z] RETRY by user:admin | status: failed | retry_count: 2 | reason: Network timeout",
        "[2023-06-15T16:20:00Z] RETRY by user:supervisor | status: success | retry_count: 3"
    ].join("\n");
    
    vm.storage_backend.as_mut().unwrap().set(
        None, 
        "governance", 
        &logs_key, 
        retry_logs.as_bytes().to_vec()
    )?;
    
    // Create metadata for the proposal to verify its existence
    let meta_key = format!("proposals/{}/metadata", proposal_id);
    vm.storage_backend.as_mut().unwrap().set(
        None,
        "governance",
        &meta_key,
        b"{\"logic_path\": \"test_logic.dsl\"}".to_vec()
    )?;
    
    // Call the handler (testing implementation, not UI output)
    let result = handle_retry_history_command(&mut vm, proposal_id);
    assert!(result.is_ok(), "Handler should not return an error");
    
    // Verify the data is correctly retrieved from storage
    let storage = vm.storage_backend.as_ref().unwrap();
    let records = storage.get_proposal_retry_history(proposal_id)?;
    
    // Verify we have 3 records
    assert_eq!(records.len(), 3);
    
    // Verify records are sorted by timestamp (newest first)
    assert_eq!(records[0].status, "success");
    assert_eq!(records[0].user, "supervisor");
    assert_eq!(records[0].retry_count, Some(3));
    
    assert_eq!(records[1].status, "failed");
    assert_eq!(records[1].retry_count, Some(2));
    assert!(records[1].reason.as_ref().unwrap().contains("Network timeout"));
    
    assert_eq!(records[2].status, "failed");
    assert_eq!(records[2].retry_count, Some(1));
    assert!(records[2].reason.as_ref().unwrap().contains("VM execution error"));
    
    Ok(())
} 