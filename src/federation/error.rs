use std::error::Error;
use std::fmt;
use std::io;
use crate::storage::errors::StorageError;

/// Error types specific to federation operations
#[derive(Debug)]
pub enum FederationError {
    /// General network error
    NetworkError(String),
    
    /// Network transport error
    TransportError(String),
    
    /// Peer connection error
    ConnectionError(String),
    
    /// Protocol error
    ProtocolError(String),
    
    /// Message serialization/deserialization error
    SerializationError(String),
    
    /// Authentication error
    AuthenticationError(String),
    
    /// Storage operation error
    StorageError(StorageError),
    
    /// Resource not found
    NotFoundError(String),

    /// Configuration error
    ConfigurationError(String),
    
    /// Clock error (for timestamp handling)
    ClockError(String),
    
    /// Permission denied
    PermissionDenied(String),
    
    /// Proposal validation error
    ProposalValidationError(String),
    
    /// Vote validation error
    VoteValidationError(String),
    
    /// Operation timeout
    TimeoutError(String),
    
    /// Generic IO errors
    IoError(io::Error),
    
    /// Invalid argument
    InvalidArgumentError(String),
    
    /// Other/unknown error
    Other(String),
}

impl fmt::Display for FederationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::TransportError(msg) => write!(f, "Transport error: {}", msg),
            Self::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            Self::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            Self::StorageError(err) => write!(f, "Storage error: {}", err),
            Self::NotFoundError(msg) => write!(f, "Not found: {}", msg),
            Self::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            Self::ClockError(msg) => write!(f, "Clock error: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::ProposalValidationError(msg) => write!(f, "Proposal validation error: {}", msg),
            Self::VoteValidationError(msg) => write!(f, "Vote validation error: {}", msg),
            Self::TimeoutError(msg) => write!(f, "Timeout: {}", msg),
            Self::IoError(err) => write!(f, "IO error: {}", err),
            Self::InvalidArgumentError(msg) => write!(f, "Invalid argument: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl Error for FederationError {}

impl From<io::Error> for FederationError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<StorageError> for FederationError {
    fn from(err: StorageError) -> Self {
        Self::StorageError(err)
    }
}

impl From<serde_json::Error> for FederationError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<std::time::SystemTimeError> for FederationError {
    fn from(err: std::time::SystemTimeError) -> Self {
        Self::ClockError(err.to_string())
    }
}

impl From<libp2p::multiaddr::Error> for FederationError {
    fn from(err: libp2p::multiaddr::Error) -> Self {
        Self::NetworkError(format!("Multiaddr error: {}", err))
    }
}
