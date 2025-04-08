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