//! Federation module for ICN-COVM networking.
//!
//! This module provides the networking layer for communication between ICN-COVM nodes,
//! allowing them to discover each other and exchange messages.

mod behaviour;
mod error;
mod events;
pub mod messages;
mod node;
pub mod storage;
#[cfg(test)]
mod tests;

pub use error::FederationError;
pub use events::NetworkEvent;
pub use messages::{
    FederatedProposal, FederatedVote, NetworkMessage, NodeAnnouncement, Ping, Pong,
};
pub use node::{NetworkNode, NodeConfig};
pub use storage::{FederationStorage, VoteTallyResult, FEDERATION_NAMESPACE, VOTES_NAMESPACE};

/// Protocol name/ID used for ICN-COVM federation
pub const PROTOCOL_ID: &str = "/icn-covm/federation/1.0.0";
