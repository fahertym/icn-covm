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

use crate::storage::errors::StorageError;
use crate::typed::TypedValueError;
use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;

/// Error variants that can occur during VM execution
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VMError {
    /// Storage backend is not available
    #[error("Storage backend not available")]
    StorageUnavailable,

    /// Invalid signature detected during operation
    #[error("Invalid signature for identity {identity_id}: {reason}")]
    InvalidSignature { identity_id: String, reason: String },

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
    #[error(
        "Memory limit exceeded: attempted {attempted_allocation} bytes, max allowed {max_allowed}"
    )]
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
    #[error("IO error: {details}")]
    IoError { details: String },

    /// An operation that is valid but not permitted by current policy
    #[error("Policy violation: {0}")]
    PolicyViolation(String),

    /// Generic storage error
    #[error("Storage error: {details}")]
    StorageError { details: String },

    /// Other/unknown error
    #[error("VM error: {0}")]
    Other(String),

    /// Operation not implemented
    #[error("Operation not implemented: {0}")]
    NotImplemented(String),

    /// Error when an assertion fails
    #[error("Assertion failed: {message}")]
    AssertionFailed { message: String },

    /// Alternative name for StorageUnavailable (for backward compatibility)
    #[error("Storage backend not available")]
    StorageNotAvailable,

    /// Error when a variable is not found
    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    /// Error when a function is not found
    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    /// Error when a parameter is not found
    #[error("Parameter not found: {0}")]
    ParameterNotFound(String),

    /// Error when identity context is not available
    #[error("Identity context not available")]
    IdentityContextUnavailable,

    /// Error when permission is denied
    #[error("Permission denied for {user} to {action} on {resource}")]
    PermissionDenied {
        user: String,
        action: String,
        resource: String,
    },

    /// Error when a type mismatch occurs
    #[error("Type mismatch in operation {operation}: expected {expected}, found {found}")]
    TypeMismatch {
        expected: String,
        found: String,
        operation: String,
    },

    /// Error when an invalid operation is attempted
    #[error("Invalid operation: {operation}")]
    InvalidOperation { operation: String },

    /// Error when a resource is not found
    #[error("Resource {resource} not found in namespace {namespace}")]
    ResourceNotFound { resource: String, namespace: String },

    /// Error when a resource already exists
    #[error("Resource {resource} already exists in namespace {namespace}")]
    ResourceAlreadyExists { resource: String, namespace: String },

    /// Error when there are insufficient funds for an operation
    #[error("Insufficient balance for account {account} in resource {resource}: required {required}, available {available}")]
    InsufficientBalance {
        resource: String,
        account: String,
        required: f64,
        available: f64,
    },

    /// Error when an invalid amount is provided
    #[error("Invalid amount: {amount}")]
    InvalidAmount { amount: f64 },

    /// Error when an identity is not found
    #[error("Identity not found: {identity_id}")]
    IdentityNotFound { identity_id: String },

    /// Error when an identity is invalid
    #[error("Invalid identity: {reason}")]
    InvalidIdentity { reason: String },

    /// Error when a storage version is not found
    #[error("Version {version} not found for key {key}")]
    VersionNotFound { key: String, version: usize },

    /// Error when configuration is invalid
    #[error("Configuration error: {details}")]
    ConfigurationError { details: String },

    /// Error when an invalid format is provided
    #[error("Invalid format: {reason}")]
    InvalidFormat { reason: String },

    /// Error when no storage backend is available
    #[error("No storage backend available")]
    NoStorageBackend,

    /// Error when serializing/deserializing data
    #[error("Serialization error: {details}")]
    SerializationError { details: String },

    /// Error from TypedValue operations
    #[error("TypedValue error: {0}")]
    #[serde(skip)]
    TypedValueError(String),

    /// Type error in VM operations
    /// 
    /// Deprecated: Use TypeMismatch instead
    #[error("Type error in {op_name}: expected {expected}, found {found}")]
    #[deprecated(since = "0.2.0", note = "Use TypeMismatch instead")]
    TypeError {
        expected: String,
        found: String,
        op_name: String,
    },

    /// Error when an undefined variable is accessed
    #[error("Undefined variable: {name}")]
    UndefinedVariable { name: String },

    /// Error when an undefined function is called
    #[error("Undefined function: {name}")]
    UndefinedFunction { name: String },

    /// Error when an undefined parameter is accessed
    #[error("Undefined parameter: {name}")]
    UndefinedParameter { name: String },
}

impl From<StorageError> for VMError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::AuthenticationError { details } => VMError::InvalidSignature {
                identity_id: "unknown".to_string(),
                reason: details,
            },
            StorageError::PermissionDenied {
                user_id,
                action,
                key,
            } => VMError::PermissionDenied {
                user: user_id,
                action,
                resource: key,
            },
            StorageError::NotFound { key } => VMError::ResourceNotFound {
                resource: key,
                namespace: "unknown".to_string(),
            },
            StorageError::ResourceNotFound(resource) => VMError::ResourceNotFound {
                resource,
                namespace: "unknown".to_string(),
            },
            StorageError::TransactionError { details } => VMError::TransactionError(details),
            StorageError::InsufficientBalance(details) => VMError::StorageError {
                details: format!("Insufficient balance: {}", details),
            },
            StorageError::ValidationError { rule, details } => VMError::ValidationError(format!(
                "Validation failed for rule '{}': {}",
                rule, details
            )),
            StorageError::IoError { operation, details } => VMError::StorageError {
                details: format!("IO error during '{}': {}", operation, details),
            },
            #[allow(deprecated)]
            StorageError::IOError { operation, details } => VMError::StorageError {
                details: format!("IO error during '{}': {}", operation, details),
            },
            StorageError::TimeError { details } => VMError::TimeError(details),
            StorageError::ConflictError { resource, details } => VMError::StorageError {
                details: format!("Conflict error on resource '{}': {}", resource, details),
            },
            StorageError::ConnectionError { backend, details } => VMError::StorageError {
                details: format!("Connection error to backend '{}': {}", backend, details),
            },
            StorageError::SerializationError { data_type, details } => {
                VMError::SerializationError {
                    details: format!("Serialization error for {}: {}", data_type, details),
                }
            }
            StorageError::VersionConflict {
                current,
                expected,
                resource,
            } => VMError::StorageError {
                details: format!(
                    "Version conflict on '{}': current {}, expected {}",
                    resource, current, expected
                ),
            },
            StorageError::ResourceLocked { resource, details } => VMError::StorageError {
                details: format!("Resource '{}' is locked: {}", resource, details),
            },
            StorageError::SchemaVersionError {
                current_version,
                required_version,
                details,
            } => VMError::StorageError {
                details: format!(
                    "Schema version error: current {}, required {}: {}",
                    current_version, required_version, details
                ),
            },
            StorageError::TimeoutError {
                operation,
                timeout_secs,
            } => VMError::TimeoutError(format!(
                "Operation '{}' timed out after {} seconds",
                operation, timeout_secs
            )),
            StorageError::QuotaExceeded {
                limit_type,
                current,
                maximum,
            } => VMError::StorageError {
                details: format!(
                    "{} quota exceeded: {} of {} used",
                    limit_type, current, maximum
                ),
            },
            StorageError::InvalidDataFormat {
                expected,
                received,
                details,
            } => VMError::InvalidFormat {
                reason: format!(
                    "Invalid data format: expected {}, received {}: {}",
                    expected, received, details
                ),
            },
            StorageError::Other { details } => VMError::Other(details),
        }
    }
}

impl From<io::Error> for VMError {
    fn from(err: io::Error) -> Self {
        VMError::IoError { details: err.to_string() }
    }
}

impl From<std::time::SystemTimeError> for VMError {
    fn from(err: std::time::SystemTimeError) -> Self {
        VMError::TimeError(err.to_string())
    }
}

impl From<serde_json::Error> for VMError {
    fn from(err: serde_json::Error) -> Self {
        VMError::Deserialization(err.to_string())
    }
}

impl From<crate::typed::TypedValueError> for VMError {
    fn from(err: crate::typed::TypedValueError) -> Self {
        match err {
            crate::typed::TypedValueError::TypeMismatch { expected, found, operation } => {
                VMError::TypeMismatch {
                    expected,
                    found,
                    operation,
                }
            },
            crate::typed::TypedValueError::InvalidOperation { operation, details } => {
                VMError::InvalidOperation {
                    operation,
                    details,
                }
            },
            crate::typed::TypedValueError::DivisionByZero => VMError::DivisionByZero,
            crate::typed::TypedValueError::OutOfBounds => VMError::InvalidOperation {
                operation: "arithmetic".to_string(),
                details: "Value out of bounds".to_string()
            },
        }
    }
}
