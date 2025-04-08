use thiserror::Error;

/// Error type for federation-related errors
#[derive(Error, Debug)]
pub enum FederationError {
    /// Error in the networking layer
    #[error("Network error: {0}")]
    NetworkError(String),
    
    /// Error serializing/deserializing messages
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    /// Error during peer discovery
    #[error("Peer discovery error: {0}")]
    DiscoveryError(String),
    
    /// Error connecting to a peer
    #[error("Peer connection error: {0}")]
    ConnectionError(String),
    
    /// Invalid message received
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
    
    /// Timeout waiting for a response
    #[error("Timeout waiting for response")]
    Timeout,
    
    /// Internal error in the libp2p implementation
    #[error("Libp2p error: {0}")]
    Libp2pError(String),
} 