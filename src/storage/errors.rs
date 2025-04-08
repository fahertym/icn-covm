use std::fmt;
use std::io;

/// Storage-related errors
#[derive(Debug, Clone)]
pub enum StorageError {
    /// Key not found in storage
    NotFound { 
        key: String 
    },
    
    /// Permission denied for an operation
    PermissionDenied { 
        user_id: String, 
        action: String, 
        key: String 
    },
    
    /// Storage quota exceeded
    QuotaExceeded { 
        account_id: String, 
        requested: u64, 
        available: u64 
    },
    
    /// Version conflict when updating a value
    VersionConflict { 
        key: String, 
        expected: u64, 
        actual: u64 
    },
    
    /// Error serializing or deserializing data
    SerializationError { 
        details: String 
    },
    
    /// Transaction-related error
    TransactionError { 
        details: String 
    },
    
    /// Error accessing underlying storage medium (IO error)
    IOError { 
        operation: String, 
        details: String 
    },
    
    /// Authentication-related errors
    AuthenticationError { 
        details: String 
    },
    
    /// Identity-related errors
    IdentityError { 
        details: String 
    },
    
    /// Invalid namespace or key format
    InvalidKey { 
        key: String, 
        details: String 
    },
    
    /// Resource limit exceeded (non-quota, e.g., max keys)
    ResourceLimitExceeded { 
        resource_type: String, 
        limit: u64, 
        attempted: u64 
    },
    
    /// Feature not implemented or available
    NotImplemented { 
        feature: String 
    },
    
    /// Resource metadata not found
    ResourceMetadataNotFound(String),
    
    /// Resource not found
    ResourceNotFound(String),
    
    /// Insufficient balance for economic operation
    InsufficientBalance(String, String),
    
    /// Invalid storage data format
    InvalidStorageData(String),
    
    /// Generic error for cases not covered by other variants
    Other { 
        details: String 
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotFound { key } => 
                write!(f, "Key not found: {}", key),
                
            StorageError::PermissionDenied { user_id, action, key } => 
                write!(f, "Permission denied for user '{}' to perform '{}' operation on '{}'", 
                       user_id, action, key),
                
            StorageError::QuotaExceeded { account_id, requested, available } => 
                write!(f, "Storage quota exceeded for account '{}': requested {} bytes, available {} bytes", 
                       account_id, requested, available),
                
            StorageError::VersionConflict { key, expected, actual } => 
                write!(f, "Version conflict on key '{}': expected version {}, got version {}", 
                       key, expected, actual),
                
            StorageError::SerializationError { details } => 
                write!(f, "Serialization error: {}", details),
                
            StorageError::TransactionError { details } => 
                write!(f, "Transaction error: {}", details),
                
            StorageError::IOError { operation, details } => 
                write!(f, "I/O error during {}: {}", operation, details),
                
            StorageError::AuthenticationError { details } => 
                write!(f, "Authentication error: {}", details),
                
            StorageError::IdentityError { details } => 
                write!(f, "Identity error: {}", details),
                
            StorageError::InvalidKey { key, details } => 
                write!(f, "Invalid key '{}': {}", key, details),
                
            StorageError::ResourceLimitExceeded { resource_type, limit, attempted } => 
                write!(f, "{} limit exceeded: limit {}, attempted {}", 
                       resource_type, limit, attempted),
                
            StorageError::NotImplemented { feature } => 
                write!(f, "Feature not implemented: {}", feature),
                
            StorageError::ResourceMetadataNotFound(resource_id) =>
                write!(f, "Resource metadata not found for resource '{}'", resource_id),
                
            StorageError::ResourceNotFound(resource_id) =>
                write!(f, "Resource not found: '{}'", resource_id),
                
            StorageError::InsufficientBalance(account, resource) =>
                write!(f, "Insufficient balance for account '{}' of resource '{}'", account, resource),
                
            StorageError::InvalidStorageData(details) =>
                write!(f, "Invalid storage data: {}", details),
                
            StorageError::Other { details } => 
                write!(f, "Storage error: {}", details),
        }
    }
}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        StorageError::IOError {
            operation: "unknown".to_string(),
            details: err.to_string(),
        }
    }
}

impl std::error::Error for StorageError {}

/// Maps an IO error to a StorageError
pub fn io_to_storage_error(operation: &str, error: std::io::Error) -> StorageError {
    StorageError::IOError {
        operation: operation.to_string(),
        details: error.to_string(),
    }
}

/// Define a standard Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;
