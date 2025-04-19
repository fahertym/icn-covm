//! Operation Handlers for VM Operations
//!
//! This module organizes VM operations by domain, allowing for better:
//! - Separation of concerns
//! - Modular operation implementation
//! - Improved testability
//! - Centralized error handling
//! - Extensibility for new operation types
//!
//! The design uses traits to define operation handlers grouped by domain:
//! - StorageOpHandler: Operations related to persistent storage
//! - GovernanceOpHandler: Operations related to governance
//! - IdentityOpHandler: Operations related to identity management
//! - ArithmeticOpHandler: Operations for arithmetic calculations
//! - ComparisonOpHandler: Operations for comparison and logical operations

use crate::identity::Identity;
use crate::storage::auth::AuthContext;
use crate::storage::errors::StorageResult;
use crate::storage::traits::Storage;
use crate::typed::TypedValue;
use crate::vm::errors::VMError;
use crate::vm::types::VMEvent;

use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Defines operations for handling storage-related VM operations
pub trait StorageOpHandler<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Set the storage backend
    fn set_storage_backend(&mut self, backend: S);

    /// Set the authentication context
    fn set_auth_context(&mut self, auth: AuthContext);

    /// Set the namespace
    fn set_namespace(&mut self, namespace: &str);

    /// Get the authentication context
    fn get_auth_context(&self) -> Option<&AuthContext>;

    /// Execute a storage operation to store a value
    fn execute_store_p(&mut self, key: &str, value: &TypedValue) -> Result<(), VMError>;

    /// Execute a storage operation to load a value
    fn execute_load_p(&mut self, key: &str, missing_key_behavior: crate::vm::MissingKeyBehavior) -> Result<TypedValue, VMError>;
}

/// Defines operations for handling governance-related VM operations
pub trait GovernanceOpHandler<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Execute a resource creation operation
    fn execute_create_resource(&mut self, resource: &str) -> Result<(), VMError>;

    /// Execute a minting operation
    fn execute_mint(
        &mut self,
        resource: &str, 
        account: &str, 
        amount: &TypedValue, 
        reason: &Option<String>
    ) -> Result<(), VMError>;

    /// Execute a transfer operation
    fn execute_transfer(
        &mut self,
        resource: &str,
        from: &str,
        to: &str,
        amount: &TypedValue,
        reason: &Option<String>,
    ) -> Result<(), VMError>;

    /// Execute a burn operation
    fn execute_burn(
        &mut self,
        resource: &str,
        account: &str,
        amount: &TypedValue,
        reason: &Option<String>,
    ) -> Result<(), VMError>;

    /// Execute a balance query operation
    fn execute_balance(&mut self, resource: &str, account: &str) -> Result<TypedValue, VMError>;
}

/// Defines operations for handling identity-related VM operations
pub trait IdentityOpHandler<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Execute increment reputation for an identity
    fn execute_increment_reputation(
        &mut self,
        identity_id: &str,
        amount: Option<&TypedValue>,
    ) -> Result<(), VMError>;

    /// Verify a signature from an identity
    fn execute_verify_identity(
        &mut self,
        identity_id: &str,
        message: &str,
        signature: &str,
    ) -> Result<bool, VMError>;

    /// Check if an identity is a member of a namespace
    fn execute_check_membership(
        &mut self,
        identity_id: &str,
        namespace: &str,
    ) -> Result<bool, VMError>;
}

/// Defines operations for arithmetic calculations
pub trait ArithmeticOpHandler {
    /// Execute arithmetic operations
    fn execute_arithmetic(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError>;
}

/// Defines operations for comparisons and logical operations
pub trait ComparisonOpHandler {
    /// Execute comparison operations
    fn execute_comparison(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError>;

    /// Execute logical operations
    fn execute_logical(&self, a: &TypedValue, op: &str) -> Result<TypedValue, VMError>;

    /// Execute binary logical operations
    fn execute_binary_logical(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError>;
}

/// Defines operations for handling VM events and output
pub trait EventHandler {
    /// Emit a message to the output
    fn emit(&mut self, message: &str);

    /// Emit an event with the given category and message
    fn emit_event(&mut self, category: &str, message: &str);

    /// Get the current output buffer
    fn get_output(&self) -> &str;

    /// Get the events as a vector
    fn get_events(&self) -> &[VMEvent];

    /// Clear the output buffer
    fn clear_output(&mut self);
}

/// Defines operations for transaction handling
pub trait TransactionHandler<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Fork the VM for transaction support
    fn fork(&mut self) -> Result<Self, VMError>
    where
        Self: Sized;

    /// Commit a transaction from a forked VM
    fn commit_fork_transaction(&mut self) -> Result<(), VMError>;

    /// Rollback a transaction from a forked VM
    fn rollback_fork_transaction(&mut self) -> Result<(), VMError>;
}

// Sub-modules implementing the traits
pub mod storage;
pub mod governance;
pub mod identity;
pub mod arithmetic;

// Re-export implementations
pub use arithmetic::ArithmeticOpImpl;
pub use governance::GovernanceOpImpl;
pub use identity::IdentityOpImpl;
pub use storage::StorageOpImpl; 