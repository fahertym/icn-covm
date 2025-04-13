#[cfg(test)]
mod tests {

    use crate::federation::messages::{NetworkMessage, NodeAnnouncement, Ping, Pong};
    use serde_json;
    use std::time::Duration;

    #[test]
    fn test_message_serialization() {
        // Create a NodeAnnouncement message
        let announcement = NodeAnnouncement {
            node_id: "node1".to_string(),
            capabilities: vec!["storage".to_string(), "execution".to_string()],
            version: "1.0.0".to_string(),
            name: Some("Test Node".to_string()),
        };

        // Wrap it in a NetworkMessage
        let message = NetworkMessage::NodeAnnouncement(announcement);

        // Serialize to JSON
        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");

        // Deserialize back
        let deserialized: NetworkMessage =
            serde_json::from_str(&serialized).expect("Failed to deserialize message");

        // Verify the result
        match deserialized {
            NetworkMessage::NodeAnnouncement(node_announcement) => {
                assert_eq!(node_announcement.node_id, "node1");
                assert_eq!(node_announcement.capabilities.len(), 2);
                assert_eq!(node_announcement.capabilities[0], "storage");
                assert_eq!(node_announcement.capabilities[1], "execution");
                assert_eq!(node_announcement.version, "1.0.0");
                assert_eq!(node_announcement.name, Some("Test Node".to_string()));
            }
            _ => panic!("Deserialized to wrong message type"),
        }
    }

    #[test]
    fn test_ping_pong_serialization() {
        // Create a Ping message
        let ping = Ping {
            nonce: 12345,
            timestamp_ms: 1618000000000,
        };

        // Wrap it in a NetworkMessage
        let message = NetworkMessage::Ping(ping);

        // Serialize to JSON
        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");

        // Deserialize back
        let deserialized: NetworkMessage =
            serde_json::from_str(&serialized).expect("Failed to deserialize message");

        // Verify the result
        match deserialized {
            NetworkMessage::Ping(ping) => {
                assert_eq!(ping.nonce, 12345);
                assert_eq!(ping.timestamp_ms, 1618000000000);
            }
            _ => panic!("Deserialized to wrong message type"),
        }

        // Test Pong message
        let pong = Pong {
            nonce: 12345,
            timestamp_ms: 1618000001000,
            ttl: Some(Duration::from_secs(60)),
        };

        // Wrap it in a NetworkMessage
        let message = NetworkMessage::Pong(pong);

        // Serialize to JSON
        let serialized = serde_json::to_string(&message).expect("Failed to serialize message");

        // Deserialize back
        let deserialized: NetworkMessage =
            serde_json::from_str(&serialized).expect("Failed to deserialize message");

        // Verify the result
        match deserialized {
            NetworkMessage::Pong(pong) => {
                assert_eq!(pong.nonce, 12345);
                assert_eq!(pong.timestamp_ms, 1618000001000);
                assert!(pong.ttl.is_some());
                assert_eq!(pong.ttl.unwrap().as_secs(), 60);
            }
            _ => panic!("Deserialized to wrong message type"),
        }
    }

    #[test]
    fn test_extract_identify_info() {
        // This is a test utility to verify that we can correctly parse listen_addrs from Identify
        let parsed_addr1: libp2p::Multiaddr = "/ip4/192.168.1.1/tcp/8000".parse().unwrap();
        let parsed_addr2: libp2p::Multiaddr = "/ip4/10.0.0.1/tcp/8001".parse().unwrap();

        // Create a Vec of addresses
        let addresses = vec![parsed_addr1.clone(), parsed_addr2.clone()];

        // Check contents
        assert_eq!(addresses.len(), 2);
        assert_eq!(addresses[0], parsed_addr1);
        assert_eq!(addresses[1], parsed_addr2);

        // Convert to strings and verify
        assert_eq!(addresses[0].to_string(), "/ip4/192.168.1.1/tcp/8000");
        assert_eq!(addresses[1].to_string(), "/ip4/10.0.0.1/tcp/8001");
    }
}

#[cfg(test)]
mod vote_tests {

    use crate::federation::messages::{ProposalScope, VotingModel};
    use crate::federation::{storage::FederationStorage, FederatedProposal, FederatedVote};
    use crate::identity::Identity;
    use crate::storage::implementations::in_memory::InMemoryStorage;
    use crate::storage::{AuthContext, StorageBackend};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn test_proposal_creation() {
        // Create a proposal
        let proposal = FederatedProposal {
            proposal_id: "test-proposal-1".to_string(),
            namespace: "test".to_string(),
            options: vec![
                "Option A".to_string(),
                "Option B".to_string(),
                "Option C".to_string(),
            ],
            creator: "test-node".to_string(),
            created_at: now(),
            expires_at: None,
            scope: ProposalScope::GlobalFederation,
            voting_model: VotingModel::OneMemberOneVote,
        };

        // Verify fields
        assert_eq!(proposal.proposal_id, "test-proposal-1");
        assert_eq!(proposal.namespace, "test");
        assert_eq!(proposal.options.len(), 3);
        assert_eq!(proposal.creator, "test-node");
    }

    #[test]
    fn test_vote_creation() {
        // Create a vote
        let vote = FederatedVote {
            proposal_id: "test-proposal-1".to_string(),
            voter: "alice".to_string(),
            ranked_choices: vec![2.0, 1.0, 0.0], // Prefers option C, then B, then A
            signature: "test-signature".to_string(),
            message: "test-vote".to_string(),
        };

        // Verify fields
        assert_eq!(vote.proposal_id, "test-proposal-1");
        assert_eq!(vote.voter, "alice");
        assert_eq!(vote.ranked_choices.len(), 3);
        assert_eq!(vote.signature, "test-signature");
    }

    #[test]
    fn test_proposal_storage() {
        // Create storage
        let mut storage = InMemoryStorage::new();
        let federation_storage = FederationStorage::new();

        // Create an admin identity with correct permissions
        let admin_identity = Identity::new("admin", "admin");

        // Create auth context with admin identity
        let mut auth = AuthContext::new("admin");
        auth.set_identity(admin_identity.clone());
        auth.add_role("global", "admin"); // Add admin role for namespace creation

        // Create account for the admin user
        storage
            .create_account(Some(&auth), "admin", 1000000)
            .unwrap();

        // Create the namespace with admin auth
        storage
            .create_namespace(Some(&auth), "test", 1000000, None)
            .unwrap();

        // Create a proposal
        let proposal = FederatedProposal {
            proposal_id: "test-proposal-1".to_string(),
            namespace: "test".to_string(),
            options: vec![
                "Option A".to_string(),
                "Option B".to_string(),
                "Option C".to_string(),
            ],
            creator: "test-node".to_string(),
            created_at: now(),
            expires_at: None,
            scope: ProposalScope::GlobalFederation,
            voting_model: VotingModel::OneMemberOneVote,
        };

        // Save the proposal with auth context
        federation_storage
            .save_proposal_with_auth(&mut storage, Some(&auth), proposal.clone())
            .unwrap();

        // Retrieve the proposal
        let retrieved_proposal = federation_storage
            .get_proposal(&storage, &proposal.proposal_id)
            .unwrap();

        // Verify fields
        assert_eq!(retrieved_proposal.proposal_id, proposal.proposal_id);
        assert_eq!(retrieved_proposal.namespace, proposal.namespace);
        assert_eq!(retrieved_proposal.options.len(), proposal.options.len());
        assert_eq!(retrieved_proposal.creator, proposal.creator);
        assert_eq!(retrieved_proposal.created_at, proposal.created_at);
    }

    #[test]
    fn test_vote_storage() {
        // Create storage
        let mut storage = InMemoryStorage::new();
        let federation_storage = FederationStorage::new();

        // Create a voter identity
        let mut identity = Identity::new("test-voter", "test-voter");
        identity.add_metadata("coop_id", "test-node"); // Match the creator cooperative ID

        // Save the proposal
        let mut auth = AuthContext::new("test-voter");
        auth.set_identity(identity.clone());
        auth.add_role("global", "admin"); // Add admin role for namespace creation

        // Create account for the test voter
        storage
            .create_account(Some(&auth), "test-voter", 1000000)
            .unwrap();

        // We need to create the namespace as the authenticated identity
        storage
            .create_namespace(Some(&auth), "test", 1000000, None)
            .unwrap();

        // Get current timestamp plus 1 hour for expiry
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let expires_at = current_time + 3600; // 1 hour in the future

        // Set up test data
        let proposal = FederatedProposal {
            proposal_id: "test-proposal".to_string(),
            namespace: "test".to_string(),
            options: vec!["Option A".to_string(), "Option B".to_string()],
            creator: "test-creator".to_string(),
            created_at: current_time,
            expires_at: Some(expires_at),
            scope: ProposalScope::GlobalFederation,
            voting_model: VotingModel::OneMemberOneVote,
        };

        // Save the proposal first with auth
        federation_storage
            .save_proposal_with_auth(&mut storage, Some(&auth), proposal.clone())
            .unwrap();

        // Create a vote
        let vote = FederatedVote {
            proposal_id: "test-proposal".to_string(),
            voter: "test-voter".to_string(),
            ranked_choices: vec![1.0, 0.0],
            message: "test vote message".to_string(),
            signature: "valid".to_string(),
        };

        // Save the vote using the authenticated identity
        federation_storage
            .save_vote(&mut storage, vote.clone(), Some(&identity))
            .unwrap();

        // Retrieve votes for the proposal
        let votes = federation_storage
            .get_votes(&storage, "test-proposal")
            .unwrap();

        // Assert that we got our vote back
        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0].voter, "test-voter");
        assert_eq!(votes[0].ranked_choices, vec![1.0, 0.0]);
    }

    #[test]
    fn test_preparing_ranked_ballots() {
        // Create federation storage
        let federation_storage = FederationStorage::new();

        // Create votes
        let votes = vec![
            FederatedVote {
                proposal_id: "test-proposal".to_string(),
                voter: "alice".to_string(),
                ranked_choices: vec![2.0, 1.0, 0.0],
                signature: "sig1".to_string(),
                message: "test-vote-1".to_string(),
            },
            FederatedVote {
                proposal_id: "test-proposal".to_string(),
                voter: "bob".to_string(),
                ranked_choices: vec![0.0, 1.0, 2.0],
                signature: "sig2".to_string(),
                message: "test-vote-2".to_string(),
            },
            FederatedVote {
                proposal_id: "test-proposal".to_string(),
                voter: "carol".to_string(),
                ranked_choices: vec![1.0, 2.0, 0.0],
                signature: "sig3".to_string(),
                message: "test-vote-3".to_string(),
            },
        ];

        // Create a test proposal with voting model
        let proposal = FederatedProposal {
            proposal_id: "test-proposal".to_string(),
            namespace: "test".to_string(),
            options: vec![
                "Option A".to_string(),
                "Option B".to_string(),
                "Option C".to_string(),
            ],
            creator: "test-node".to_string(),
            created_at: 0,
            expires_at: None,
            scope: ProposalScope::GlobalFederation,
            voting_model: VotingModel::OneMemberOneVote,
        };

        // Create voter identities (empty for this test)
        let voter_identities = std::collections::HashMap::new();

        // Convert to ballots
        let ballots =
            federation_storage.prepare_ranked_ballots(&votes, &proposal, &voter_identities);

        // Verify
        assert_eq!(ballots.len(), 3);
        assert_eq!(ballots[0], vec![2.0, 1.0, 0.0]);
        assert_eq!(ballots[1], vec![0.0, 1.0, 2.0]);
        assert_eq!(ballots[2], vec![1.0, 2.0, 0.0]);
    }
}
