//! Error types for VM operations
//!
//! This module defines all possible error conditions that can occur during VM execution.

use thiserror::Error;

/// Error variants that can occur during VM execution
#[derive(Debug, Error, Clone, PartialEq)]
pub enum VMError {
    /// Stack underflow occurs when trying to pop more values than are available
    #[error("Stack underflow during {op_name}")]
    StackUnderflow { op_name: String },

    /// Division by zero error
    #[error("Division by zero")]
    DivisionByZero,

    /// Error when a variable is not found in memory
    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    /// Error when a function is not found
    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    /// Error when maximum recursion depth is exceeded
    #[error("Maximum recursion depth exceeded")]
    MaxRecursionDepth,

    /// Error when a condition expression is invalid
    #[error("Invalid condition: {0}")]
    InvalidCondition(String),

    /// Error when an assertion fails
    #[error("Assertion failed: {message}")]
    AssertionFailed { message: String },

    /// I/O error during execution
    #[error("IO error: {0}")]
    IOError(String),

    /// Error in the REPL
    #[error("REPL error: {0}")]
    ReplError(String),

    /// Error with parameter handling
    #[error("Parameter error: {0}")]
    ParameterError(String),

    /// Loop control signal (break/continue)
    #[error("Loop control: {0}")]
    LoopControl(String),

    /// Feature not implemented
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Storage-related error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Storage backend is unavailable or not configured
    #[error("Storage backend is unavailable or not configured")]
    StorageUnavailable,

    /// Parameter not found
    #[error("Parameter not found: {0}")]
    ParameterNotFound(String),

    /// Identity not found
    #[error("Identity not found: {0}")]
    IdentityNotFound(String),

    /// Invalid signature
    #[error("Invalid signature for identity {identity_id}: {reason}")]
    InvalidSignature { identity_id: String, reason: String },

    /// Membership check failed
    #[error("Membership check failed for identity {identity_id} in namespace {namespace}")]
    MembershipCheckFailed {
        identity_id: String,
        namespace: String,
    },

    /// Delegation check failed
    #[error("Delegation check failed from {delegator_id} to {delegate_id}")]
    DelegationCheckFailed {
        delegator_id: String,
        delegate_id: String,
    },

    /// Identity context unavailable
    #[error("Identity context unavailable")]
    IdentityContextUnavailable,

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),
} 