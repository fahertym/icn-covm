use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Core message types for node communication in the federation network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Announcement of a node joining the network
    NodeAnnouncement(NodeAnnouncement),
    
    /// Ping message to verify node connectivity
    Ping(Ping),
    
    /// Pong response to a ping message
    Pong(Pong),
    
    /// Broadcast a proposal to the federation network
    ProposalBroadcast(FederatedProposal),
    
    /// Submit a vote for a federated proposal
    VoteSubmission(FederatedVote),
}

/// Message announcing a node's presence and capabilities on the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAnnouncement {
    /// Unique identifier for the node
    pub node_id: String,
    
    /// List of capabilities supported by this node
    pub capabilities: Vec<String>,
    
    /// Version information for the node software
    pub version: String,
    
    /// Optional human-readable name for this node
    pub name: Option<String>,
}

/// Ping message used to verify node connectivity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping {
    /// Random nonce value used to correlate ping/pong pairs
    pub nonce: u64,
    
    /// Timestamp when ping was sent (useful for latency calculation)
    pub timestamp_ms: u64,
}

/// Pong message sent in response to a Ping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pong {
    /// Nonce from the original ping message (for correlation)
    pub nonce: u64,
    
    /// Timestamp when the pong was sent
    pub timestamp_ms: u64,
    
    /// Optional time-to-live for this node's connection
    pub ttl: Option<Duration>,
}

/// Defines the scope of a proposal and which cooperatives can participate in voting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalScope {
    /// Only members of the specified cooperative can vote
    SingleCoop(String),
    
    /// Only members of the listed cooperatives can vote
    MultiCoop(Vec<String>),
    
    /// All federation members can vote regardless of cooperative
    GlobalFederation,
}

/// Defines how votes are counted for a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotingModel {
    /// Each member gets one vote (traditional direct democracy)
    OneMemberOneVote,
    
    /// Each cooperative gets one vote (federated representation)
    OneCoopOneVote,
}

/// Proposal for federation-wide voting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedProposal {
    /// Unique identifier for the proposal
    pub proposal_id: String,
    
    /// Namespace in which this proposal belongs
    pub namespace: String,
    
    /// Available voting options for this proposal
    pub options: Vec<String>,
    
    /// Identifier of the proposal creator
    pub creator: String,
    
    /// Timestamp when the proposal was created
    pub created_at: i64,
    
    /// Scope defining which cooperatives can participate in voting
    pub scope: ProposalScope,
    
    /// Model defining how votes are counted
    pub voting_model: VotingModel,
}

/// Vote on a federated proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedVote {
    /// Unique identifier of the proposal being voted on
    pub proposal_id: String,
    
    /// Identifier of the voter
    pub voter: String,
    
    /// Ranked preferences for each option (preference values)
    pub ranked_choices: Vec<f64>,
    
    /// The canonical message that was signed
    pub message: String,
    
    /// Signature to verify the vote's authenticity
    pub signature: String,
} 