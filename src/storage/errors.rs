use std::fmt;
use std::io;

// Storage errors
#[derive(Debug, Clone)]
pub enum StorageError {
    NotFound {
        key: String,
    },
    PermissionDenied {
        user_id: String,
        action: String,
        key: String,
    },
    QuotaExceeded {
        account_id: String,
        requested: u64,
        available: u64,
    },
    VersionConflict {
        key: String,
        expected: u64,
        actual: u64,
    },
    SerializationError {
        details: String,
    },
    TransactionError {
        details: String,
    },
    IoError {
        details: String,
    },
    // Economic operation errors
    InvalidOperation(String),
    InsufficientBalance {
        account_id: String,
        resource_id: String,
        requested: f64,
        available: f64,
    },
    ResourceNotFound {
        resource_id: String,
    },
    AccountNotFound {
        account_id: String,
    },
    // Add other specific errors as needed
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotFound { key } => write!(f, "Key not found: {}", key),
            StorageError::PermissionDenied {
                user_id,
                action,
                key,
            } => write!(
                f,
                "Permission denied for user {} to {} on key {}",
                user_id, action, key
            ),
            StorageError::QuotaExceeded {
                account_id,
                requested,
                available,
            } => write!(
                f,
                "Storage quota exceeded for {}: requested {} bytes, available {} bytes",
                account_id, requested, available
            ),
            StorageError::VersionConflict {
                key,
                expected,
                actual,
            } => write!(
                f,
                "Version conflict on key {}: expected {}, got {}",
                key, expected, actual
            ),
            StorageError::SerializationError { details } => {
                write!(f, "Serialization error: {}", details)
            }
            StorageError::TransactionError { details } => {
                write!(f, "Transaction error: {}", details)
            }
            StorageError::IoError { details } => write!(f, "I/O error: {}", details),
            StorageError::InvalidOperation(details) => write!(f, "Invalid operation: {}", details),
            StorageError::InsufficientBalance {
                account_id,
                resource_id,
                requested,
                available,
            } => write!(
                f,
                "Insufficient balance for account {}, resource {}: requested {}, available {}",
                account_id, resource_id, requested, available
            ),
            StorageError::ResourceNotFound { resource_id } => {
                write!(f, "Resource not found: {}", resource_id)
            }
            StorageError::AccountNotFound { account_id } => {
                write!(f, "Account not found: {}", account_id)
            }
        }
    }
}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        StorageError::IoError {
            details: err.to_string(),
        }
    }
}

impl std::error::Error for StorageError {}

// Define a standard Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;
