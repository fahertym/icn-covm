// Parse the timestamp from the record
if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(&record.timestamp) {
    let utc_timestamp = timestamp.with_timezone(&chrono::Utc);
    let remaining = time::get_cooldown_remaining(&utc_timestamp, COOLDOWN_DURATION);
    
    if remaining.num_seconds() > 0 {
        println!("⏳ Cooldown: Wait {} before next retry allowed", 
                    time::format_duration(remaining));
    } else {
        println!("✅ Cooldown: Ready for next retry");
    }
} 

#[cfg(test)]
fn setup_test_vm() -> VM<InMemoryStorage> {
    let storage = InMemoryStorage::new();
    VM::with_storage_backend(storage)
}

#[cfg(test)]
fn setup_test_auth() -> AuthContext {
    let mut auth = AuthContext::new("did:key:test");
    auth.add_role("governance", "admin");
    auth
}

#[cfg(test)]
fn create_test_proposal<S>(vm: &mut VM<S>, proposal_id: &str) -> Result<(), Box<dyn Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let proposal_key = format!("proposals/{}/metadata", proposal_id);
    
    // Create a simplified test identity
    let creator = "did:key:test".to_string();
    
    // Create a proposal lifecycle with required parameters
    let proposal = crate::governance::proposal_lifecycle::ProposalLifecycle::new(
        proposal_id.to_string(),
        creator,
        "Test Proposal".to_string(),
        55, // quorum percentage
        60, // threshold percentage
        None, // discussion_duration
        None, // required_participants
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

    let now = Utc::now();
    let comment = ProposalComment {
        id: comment_id.to_string(),
        author: auth.get_identity_did().to_string(),
        timestamp: now,
        content: "This is a test comment".to_string(),
        reply_to: None,
        tags: Vec::new(),
        reactions,
        hidden: false,
        edit_history: vec![crate::governance::comments::CommentVersion {
            content: "This is a test comment".to_string(),
            timestamp: now,
        }],
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
    
    if let Some(storage) = vm.storage_backend.as_mut() {
        storage.set(
            None, 
            "governance", 
            &logs_key, 
            retry_logs.as_bytes().to_vec()
        )?;
        
        // Create metadata for the proposal to verify its existence
        let meta_key = format!("proposals/{}/metadata", proposal_id);
        storage.set(
            None,
            "governance",
            &meta_key,
            b"{\"logic_path\": \"test_logic.dsl\"}".to_vec()
        )?;
    }
    
    // Call the handler (testing implementation, not UI output)
    let result = handle_retry_history_command(&mut vm, proposal_id);
    assert!(result.is_ok(), "Handler should not return an error");
    
    // Verify the data is correctly retrieved from storage
    if let Some(storage) = vm.storage_backend.as_ref() {
        if let Ok(records) = storage.get_proposal_retry_history(proposal_id) {
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
        }
    }
    
    Ok(())
} 