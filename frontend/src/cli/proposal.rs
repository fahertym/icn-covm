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