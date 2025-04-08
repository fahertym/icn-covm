//! Federation module for ICN-COVM networking.
//! 
//! This module provides the networking layer for communication between ICN-COVM nodes,
//! allowing them to discover each other and exchange messages.

mod node;
mod messages;
mod error;
mod behaviour;
mod events;
#[cfg(test)]
mod tests;

pub use node::{NetworkNode, NodeConfig};
pub use messages::{NetworkMessage, NodeAnnouncement, Ping, Pong};
pub use error::FederationError;
pub use events::NetworkEvent;

/// Protocol name/ID used for ICN-COVM federation
pub const PROTOCOL_ID: &str = "/icn-covm/federation/1.0.0"; 