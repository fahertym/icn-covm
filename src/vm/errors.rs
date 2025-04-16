//! Error types for VM operations
//!
//! This module defines all possible error conditions that can occur during VM execution.
//!
//! Having a dedicated error module provides:
//! - Consistent error handling throughout the VM
//! - Clear categorization of different error types
//! - Detailed error messages with relevant context
//! - Extensibility for adding new error variants
//! - Better integration with Rust's error handling patterns
//!
//! The error types are designed to:
//! - Be descriptive about what went wrong
//! - Include relevant context (e.g., operation name, variable name)
//! - Enable proper error propagation in the VM
//! - Support clean error reporting to users

use thiserror::Error;
use crate::storage::errors::StorageError;
use std::fmt;
use std::io;

/// Error variants that can occur during VM execution
#[derive(Error, Debug)]
pub enum VMError {
    /// Storage backend is not available
    #[error("Storage backend not available")]
    StorageUnavailable,

    /// Invalid signature detected during operation
    #[error("Invalid signature for identity {identity_id}: {reason}")]
    InvalidSignature {
        identity_id: String,
        reason: String,
    },

    /// Undefined operation encountered
    #[error("Undefined operation: {0}")]
    UndefinedOperation(String),

    /// Division by zero attempted
    #[error("Division by zero")]
    DivisionByZero,

    /// Stack underflow (pop from empty stack)
    #[error("Stack underflow")]
    StackUnderflow,

    /// Register access error
    #[error("Register error: {0}")]
    RegisterError(String),

    /// Invalid bytecode format
    #[error("Invalid bytecode: {0}")]
    InvalidBytecode(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Error when a transaction operation fails
    #[error("Transaction error: {0}")]
    TransactionError(String),

    /// Error when an operation uses invalid syntax or arguments
    #[error("Syntax error: {0}")]
    SyntaxError(String),

    /// Error when a test validation fails
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Error when attempting an arithmetic operation that would be invalid
    #[error("Arithmetic error: {0}")]
    ArithmeticError(String),

    /// Error when a resource does not exist
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Error when an account does not exist
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// Error when program execution reaches an undefined state
    #[error("Undefined state: {0}")]
    UndefinedState(String),

    /// Error when VM execution reaches the maximum step limit
    #[error("Step limit exceeded: {0} steps")]
    StepLimitExceeded(usize),

    /// Error when VM execution reaches the maximum stack depth
    #[error("Stack overflow at depth {0}")]
    StackOverflow(usize),

    /// Error when a namespace operation fails
    #[error("Namespace error: {0}")]
    NamespaceError(String),

    /// Error when authorization fails
    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    /// Error when a governance operation fails
    #[error("Governance error: {0}")]
    GovernanceError(String),

    /// Error when a parsing operation fails
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Error when a VM operation is executed in the wrong context
    #[error("Context mismatch: {0}")]
    ContextMismatch(String),

    /// Error when a memory operation exceeds limits
    #[error("Memory limit exceeded: attempted {attempted_allocation} bytes, max allowed {max_allowed}")]
    MemoryLimitExceeded {
        attempted_allocation: usize,
        max_allowed: usize,
    },

    /// Error when a loop exceeds its iteration limit
    #[error("Loop limit exceeded: {iterations} iterations, max allowed {max_allowed}")]
    LoopLimitExceeded {
        iterations: usize,
        max_allowed: usize,
    },

    /// Error when an operation timed out
    #[error("Timeout: {0}")]
    TimeoutError(String),

    /// Error from a clock or time-related operation
    #[error("Time error: {0}")]
    TimeError(String),

    /// IO error during VM operation
    #[error("IO error: {0}")]
    IoError(io::Error),

    /// An operation that is valid but not permitted by current policy
    #[error("Policy violation: {0}")]
    PolicyViolation(String),

    /// Generic storage error
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Other/unknown error
    #[error("VM error: {0}")]
    Other(String),
}

impl From<StorageError> for VMError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::AuthenticationError { details } => VMError::AuthorizationError(details),
            StorageError::PermissionDenied {
                user_id,
                action,
                key,
            } => VMError::AuthorizationError(format!(
                "Permission denied for user '{}' to perform '{}' on '{}'",
                user_id, action, key
            )),
            StorageError::NotFound { key } => VMError::StorageError(format!("Key not found: {}", key)),
            StorageError::TransactionError { details } => VMError::TransactionError(details),
            StorageError::ConflictError { resource, details } => 
                VMError::StorageError(format!("Conflict error on resource '{}': {}", resource, details)),
            StorageError::ConnectionError { backend, details } => 
                VMError::StorageError(format!("Connection error to backend '{}': {}", backend, details)),
            StorageError::SerializationError { data_type, details } => 
                VMError::Deserialization(format!("Serialization error for {}: {}", data_type, details)),
            StorageError::InvalidDataFormat { expected, received, details } => 
                VMError::ParseError(format!("Invalid data format: expected {}, received {}: {}", expected, received, details)),
            StorageError::QuotaExceeded { limit_type, current, maximum } => 
                VMError::StorageError(format!("{} quota exceeded: {} of {} used", limit_type, current, maximum)),
            StorageError::TimeoutError { operation, timeout_secs } => 
                VMError::TimeoutError(format!("Operation '{}' timed out after {} seconds", operation, timeout_secs)),
            StorageError::ResourceLocked { resource, details } => 
                VMError::StorageError(format!("Resource '{}' is locked: {}", resource, details)),
            StorageError::ValidationError { rule, details } => 
                VMError::ValidationError(format!("Validation failed for rule '{}': {}", rule, details)),
            StorageError::IoError { operation, details } => 
                VMError::IoError(io::Error::new(io::ErrorKind::Other, details)),
            StorageError::TimeError { details } => VMError::TimeError(details),
            StorageError::SchemaVersionError { current_version, required_version, details } => 
                VMError::StorageError(format!("Schema version error: current {}, required {}: {}", 
                    current_version, required_version, details)),
            StorageError::Other { details } => VMError::StorageError(details),
        }
    }
}

impl From<io::Error> for VMError {
    fn from(err: io::Error) -> Self {
        VMError::IoError(err)
    }
}

impl From<std::time::SystemTimeError> for VMError {
    fn from(err: std::time::SystemTimeError) -> Self {
        VMError::TimeError(format!("System time error: {}", err))
    }
}

impl From<serde_json::Error> for VMError {
    fn from(err: serde_json::Error) -> Self {
        VMError::ParseError(format!("JSON error: {}", err))
    }
}
