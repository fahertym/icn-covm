use crate::storage::auth::AuthContext;
use crate::storage::errors::StorageResult;
use crate::storage::versioning::VersionInfo;
use crate::storage::events::StorageEvent;
use serde::{Serialize, de::DeserializeOwned};

/// Defines the core operations for a cooperative storage backend.
/// This trait is designed to be object-safe where possible, but some methods
/// returning complex types or involving generics might require specific handling.
pub trait StorageBackend {
    /// Retrieves raw byte data associated with a key within a namespace.
    /// Performs permission checks based on the provided `AuthContext`.
    fn get(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<Vec<u8>>;

    /// Retrieves data along with its versioning information.
    fn get_versioned(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<(Vec<u8>, VersionInfo)>;

    /// Sets raw byte data for a key within a namespace.
    /// Performs permission checks and resource accounting.
    /// Updates version information.
    fn set(&mut self, auth: &AuthContext, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()>;

    /// Creates a resource account for a user.
    /// Typically requires administrative privileges.
    fn create_account(&mut self, auth: &AuthContext, user_id: &str, quota_bytes: u64) -> StorageResult<()>;

    /// Checks if the user has the required permission for an action in a namespace.
    /// This might be used internally by other methods or exposed for direct checks.
    fn check_permission(&self, auth: &AuthContext, action: &str, namespace: &str) -> StorageResult<()>;

    /// Begins a transaction.
    /// Subsequent `set` operations should be part of this transaction until commit/rollback.
    fn begin_transaction(&mut self) -> StorageResult<()>;

    /// Commits the current transaction, making changes permanent.
    fn commit_transaction(&mut self) -> StorageResult<()>;

    /// Rolls back the current transaction, discarding changes.
    fn rollback_transaction(&mut self) -> StorageResult<()>;

    /// Retrieves audit log entries, potentially filtered.
    /// Requires appropriate permissions.
    fn get_audit_log(&self, auth: &AuthContext, namespace: Option<&str>, event_type: Option<&str>, limit: usize) -> StorageResult<Vec<StorageEvent>>;

    // TODO: Add methods for deletion, listing keys, managing roles/delegations directly?
}

// Potential trait for federated operations (optional for now)
// pub trait FederatedStorageBackend: StorageBackend {
//     fn push(&self, remote_target: &str, namespace: &str) -> StorageResult<()>;
//     fn pull(&mut self, remote_source: &str, namespace: &str) -> StorageResult<()>;
// }
