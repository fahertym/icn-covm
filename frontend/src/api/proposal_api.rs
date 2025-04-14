/// Counts votes for a proposal
///
/// Returns a tuple of (yes_votes, no_votes, abstain_votes)
fn count_votes<S>(vm: &VM<S>, id: &str) -> Result<(u32, u32, u32), String>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm.storage_backend.as_ref().unwrap();
    let votes_key = format!("proposals/{}/votes", id);
    
    let mut yes_votes = 0;
    let mut no_votes = 0;
    let mut abstain_votes = 0;
    
    // Get votes if available
    if let Ok(votes_data) = storage.get(None, "governance", &votes_key) {
        if let Ok(votes_map) = serde_json::from_slice::<HashMap<String, String>>(&votes_data) {
            for vote_type in votes_map.values() {
                match vote_type.as_str() {
                    "yes" => yes_votes += 1,
                    "no" => no_votes += 1,
                    "abstain" => abstain_votes += 1,
                    _ => {}
                }
            }
        }
    }

    Ok((yes_votes, no_votes, abstain_votes))
}

/// Loads a proposal from storage 
fn load_proposal_from_governance<S>(vm: &VM<S>, id: &str) -> Result<Proposal, String>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    let storage = vm.storage_backend.as_ref().unwrap();
    let proposal_key = format!("proposals/{}/metadata", id);
    
    match storage.get(None, "governance", &proposal_key) {
        Ok(data) => {
            match serde_json::from_slice::<ProposalLifecycle>(&data) {
                Ok(lifecycle) => {
                    // Convert the lifecycle to a proposal
                    let status: ProposalStatus = lifecycle.state.into();
                    let creator = lifecycle.creator.id.to_string(); // Convert Identity to String by using its ID
                    
                    let proposal = Proposal {
                        id: lifecycle.id.clone(),
                        creator,
                        status,
                        created_at: lifecycle.created_at,
                        expires_at: lifecycle.expires_at,
                        logic_path: None, // This would need to be fetched separately
                        discussion_path: None, // This would need to be fetched separately
                        votes_path: None, // This would need to be fetched separately
                        attachments: Vec::new(), // Attachments would need to be fetched separately
                        execution_result: lifecycle.execution_status.map(|status| format!("{:?}", status)),
                        deliberation_started_at: None, // Not available in lifecycle directly
                        min_deliberation_hours: None, // Not available in lifecycle directly
                    };
                    Ok(proposal)
                },
                Err(e) => Err(format!("Failed to deserialize proposal: {}", e)),
            }
        },
        Err(e) => Err(format!("Failed to retrieve proposal: {}", e)),
    }
} 