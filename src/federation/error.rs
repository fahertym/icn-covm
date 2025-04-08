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

impl From<libp2p::multiaddr::Error> for FederationError {
    fn from(err: libp2p::multiaddr::Error) -> Self {
        FederationError::NetworkError(format!("Multiaddr error: {}", err))
    }
}

// Add more conversions for libp2p errors
impl From<std::io::Error> for FederationError {
    fn from(err: std::io::Error) -> Self {
        FederationError::NetworkError(format!("I/O error: {}", err))
    }
}

impl From<libp2p::TransportError<std::io::Error>> for FederationError {
    fn from(err: libp2p::TransportError<std::io::Error>) -> Self {
        FederationError::NetworkError(format!("Transport error: {}", err))
    }
}

impl From<libp2p::swarm::DialError> for FederationError {
    fn from(err: libp2p::swarm::DialError) -> Self {
        FederationError::ConnectionError(format!("Dial error: {}", err))
    }
} 