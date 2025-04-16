use std::fmt;
use std::io;

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Error types for storage operations
#[derive(Debug, Clone)]
pub enum StorageError {
    /// Authentication error
    AuthenticationError {
        /// Details about the authentication error
        details: String,
    },

    /// Permission denied for an operation
    PermissionDenied {
        /// User ID that was denied
        user_id: String,
        /// Action that was attempted
        action: String,
        /// Key that was accessed
        key: String,
    },

    /// Key not found in storage
    NotFound {
        /// Key that was not found
        key: String,
    },

    /// Transaction error
    TransactionError {
        /// Details about the transaction error
        details: String,
    },
    
    /// Conflict when modifying a resource
    ConflictError {
        /// Resource that had a conflict
        resource: String,
        /// Details about the conflict
        details: String,
    },
    
    /// Backend connection error
    ConnectionError {
        /// Backend identifier
        backend: String,
        /// Details about the connection error
        details: String,
    },
    
    /// Serialization or deserialization error
    SerializationError {
        /// Type being serialized/deserialized
        data_type: String,
        /// Details about the serialization error
        details: String,
    },
    
    /// Invalid data format
    InvalidDataFormat {
        /// Expected format
        expected: String,
        /// Received format
        received: String,
        /// Additional details
        details: String,
    },
    
    /// Quota or limit exceeded
    QuotaExceeded {
        /// Limit that was exceeded
        limit_type: String,
        /// Current usage
        current: u64,
        /// Maximum allowed
        maximum: u64,
    },
    
    /// Operation timeout
    TimeoutError {
        /// Operation that timed out
        operation: String,
        /// Timeout duration in seconds
        timeout_secs: u64,
    },
    
    /// Resource locked by another operation
    ResourceLocked {
        /// Resource that is locked
        resource: String,
        /// Details about the lock
        details: String,
    },
    
    /// Backend-specific validation error
    ValidationError {
        /// Validation rule that failed
        rule: String,
        /// Details about the validation error
        details: String,
    },
    
    /// IO error during storage operation
    IoError {
        /// Details about the operation that failed
        operation: String,
        /// Error message
        details: String,
    },
    
    /// Clock or timestamp error
    TimeError {
        /// Details about the time error
        details: String,
    },
    
    /// Migration or schema version error
    SchemaVersionError {
        /// Current schema version
        current_version: String,
        /// Required schema version
        required_version: String,
        /// Details about the version error
        details: String,
    },
    
    /// Other or unknown error
    Other {
        /// Details about the error
        details: String,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthenticationError { details } => {
                write!(f, "Authentication error: {}", details)
            }
            Self::PermissionDenied { user_id, action, key } => {
                write!(f, "Permission denied for user '{}' to perform '{}' on '{}'", user_id, action, key)
            }
            Self::NotFound { key } => {
                write!(f, "Key not found: {}", key)
            }
            Self::TransactionError { details } => {
                write!(f, "Transaction error: {}", details)
            }
            Self::ConflictError { resource, details } => {
                write!(f, "Conflict error on resource '{}': {}", resource, details)
            }
            Self::ConnectionError { backend, details } => {
                write!(f, "Connection error to backend '{}': {}", backend, details)
            }
            Self::SerializationError { data_type, details } => {
                write!(f, "Serialization error for {}: {}", data_type, details)
            }
            Self::InvalidDataFormat { expected, received, details } => {
                write!(f, "Invalid data format: expected {}, received {}: {}", expected, received, details)
            }
            Self::QuotaExceeded { limit_type, current, maximum } => {
                write!(f, "{} quota exceeded: {} of {} used", limit_type, current, maximum)
            }
            Self::TimeoutError { operation, timeout_secs } => {
                write!(f, "Operation '{}' timed out after {} seconds", operation, timeout_secs)
            }
            Self::ResourceLocked { resource, details } => {
                write!(f, "Resource '{}' is locked: {}", resource, details)
            }
            Self::ValidationError { rule, details } => {
                write!(f, "Validation failed for rule '{}': {}", rule, details)
            }
            Self::IoError { operation, details } => {
                write!(f, "IO error: {}", details)
            }
            Self::TimeError { details } => {
                write!(f, "Time error: {}", details)
            }
            Self::SchemaVersionError { current_version, required_version, details } => {
                write!(f, "Schema version error: current {}, required {}: {}", 
                    current_version, required_version, details)
            }
            Self::Other { details } => {
                write!(f, "Storage error: {}", details)
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        Self::IoError {
            operation: "unknown".to_string(),
            details: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError {
            data_type: "JSON".to_string(),
            details: err.to_string(),
        }
    }
}

impl From<std::time::SystemTimeError> for StorageError {
    fn from(err: std::time::SystemTimeError) -> Self {
        Self::TimeError {
            details: format!("System time error: {}", err),
        }
    }
}
