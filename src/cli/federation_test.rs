#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::messages::{FederatedProposal, ProposalScope, ProposalStatus, VotingModel};
    use crate::governance::proposal::{Proposal, ProposalStatus as LocalProposalStatus};
    use crate::governance::proposal_lifecycle::VoteChoice;
    use crate::storage::auth::AuthContext;
    use crate::storage::implementations::in_memory::InMemoryStorage;
    use crate::vm::VM;
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;

    // Helper function to create a test VM with in-memory storage
    fn setup_test_vm() -> VM<InMemoryStorage> {
        let storage = InMemoryStorage::new();
        VM::with_storage_backend(storage)
    }

    // Helper function to create a test auth context
    fn setup_test_auth() -> AuthContext {
        let mut memberships = Vec::new();
        memberships.push("test-coop-1".to_string());
        
        AuthContext {
            current_identity_did: "test-user-1".to_string(),
            identity_registry: HashMap::new(),
            roles: HashMap::new(),
            memberships,
            delegations: HashMap::new(),
        }
    }

    // Helper function to create a test proposal
    fn create_test_proposal() -> Proposal {
        Proposal {
            id: "test-proposal-1".to_string(),
            creator: "test-user-1".to_string(),
            status: LocalProposalStatus::Voting,
            created_at: Utc::now(),
            expires_at: None,
            logic_path: Some("test_logic.dsl".to_string()),
            discussion_path: Some("discussion/test-proposal-1".to_string()),
            votes_path: Some("votes/test-proposal-1".to_string()),
            attachments: vec![],
            execution_result: None,
            deliberation_started_at: Some(Utc::now()),
            min_deliberation_hours: Some(24),
        }
    }
    
    // Test converting a local proposal to a federated proposal
    #[test]
    fn test_local_to_federated_conversion() {
        // Create a local proposal
        let local_proposal = create_test_proposal();
        
        // Convert to federated proposal
        let scope = ProposalScope::SingleCoop("test-coop-1".to_string());
        let voting_model = VotingModel::OneMemberOneVote;
        let federated = local_to_federated_proposal(&local_proposal, scope, voting_model, None);
        
        // Verify conversion
        assert_eq!(federated.proposal_id, local_proposal.id);
        assert_eq!(federated.creator, local_proposal.creator);
        assert_eq!(federated.status, ProposalStatus::Open);
        assert_eq!(federated.namespace, "governance");
        assert_eq!(federated.options, vec!["Yes".to_string(), "No".to_string()]);
        
        // Check scope
        match federated.scope {
            ProposalScope::SingleCoop(coop_id) => {
                assert_eq!(coop_id, "test-coop-1");
            },
            _ => panic!("Expected SingleCoop scope"),
        }
        
        // Check voting model
        match federated.voting_model {
            VotingModel::OneMemberOneVote => {},
            _ => panic!("Expected OneMemberOneVote model"),
        }
    }
    
    // Test conversion with expiration
    #[test]
    fn test_conversion_with_expiration() {
        // Create a local proposal
        let local_proposal = create_test_proposal();
        
        // Convert to federated proposal with expiration
        let scope = ProposalScope::SingleCoop("test-coop-1".to_string());
        let voting_model = VotingModel::OneMemberOneVote;
        let expires_in = Some(3600u64); // 1 hour
        
        let federated = local_to_federated_proposal(&local_proposal, scope, voting_model, expires_in);
        
        // Verify expiration was set
        assert!(federated.expires_at.is_some());
        
        // Expiration should be ~1 hour in the future from now
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        let expires = federated.expires_at.unwrap();
        let diff = expires - now;
        
        // Allow for a small timing difference due to test execution
        assert!(diff > 3500 && diff <= 3700);
    }
    
    // Test vote choice conversion
    #[test]
    fn test_vote_choice_conversion() {
        // Test Yes vote
        let yes_choices = vote_choice_to_ranked_choices(&VoteChoice::Yes);
        assert_eq!(yes_choices, vec![1.0, 0.0]);
        
        // Test No vote
        let no_choices = vote_choice_to_ranked_choices(&VoteChoice::No);
        assert_eq!(no_choices, vec![0.0, 1.0]);
        
        // Test Abstain vote
        let abstain_choices = vote_choice_to_ranked_choices(&VoteChoice::Abstain);
        assert_eq!(abstain_choices, vec![0.0, 0.0]);
    }
    
    // Test storing and retrieving a federated proposal
    #[tokio::test]
    async fn test_store_federated_proposal() {
        let mut vm = setup_test_vm();
        let auth_context = setup_test_auth();
        
        // Create a federated proposal
        let federated_proposal = FederatedProposal {
            proposal_id: "fed-test-1".to_string(),
            namespace: "governance".to_string(),
            options: vec!["Yes".to_string(), "No".to_string()],
            creator: "test-user-1".to_string(),
            created_at: Utc::now().timestamp(),
            scope: ProposalScope::SingleCoop("test-coop-1".to_string()),
            voting_model: VotingModel::OneMemberOneVote,
            expires_at: None,
            status: ProposalStatus::Open,
        };
        
        // Store the proposal
        let mut storage = vm.get_storage_backend().unwrap().clone();
        let storage_key = format!("{}/{}", FEDERATION_PROPOSALS_PATH, federated_proposal.proposal_id);
        let proposal_data = serde_json::to_vec(&federated_proposal).unwrap();
        
        storage.set(
            Some(&auth_context), 
            "federation", 
            &storage_key, 
            proposal_data
        ).unwrap();
        
        // Retrieve the proposal
        let storage_ref = vm.get_storage_backend().unwrap();
        let retrieved_data = storage_ref.get(
            Some(&auth_context), 
            "federation", 
            &storage_key
        ).unwrap();
        let retrieved: FederatedProposal = serde_json::from_slice(&retrieved_data).unwrap();
        
        // Verify
        assert_eq!(retrieved.proposal_id, federated_proposal.proposal_id);
        assert_eq!(retrieved.creator, federated_proposal.creator);
        assert_eq!(retrieved.status, federated_proposal.status);
    }
} 