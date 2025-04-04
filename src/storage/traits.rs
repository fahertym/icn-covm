use crate::storage::auth::AuthContext;
use crate::storage::errors::StorageResult;
use crate::storage::events::StorageEvent;
use crate::storage::namespaces::NamespaceMetadata;
use crate::storage::versioning::{VersionDiff, VersionInfo};
use serde::{de::DeserializeOwned, Serialize};

/// Defines the core operations for a cooperative storage backend.
/// This trait is designed to be object-safe where possible, but some methods
/// returning complex types or involving generics might require specific handling.
pub trait StorageBackend {
    /// Retrieves raw byte data associated with a key within a namespace.
    /// Performs permission checks based on the provided `AuthContext`.
    fn get(&self, auth: Option<&AuthContext>, namespace: &str, key: &str)
        -> StorageResult<Vec<u8>>;

    /// Retrieves data along with its versioning information.
    fn get_versioned(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<(Vec<u8>, VersionInfo)>;

    /// Retrieves a specific version of data
    fn get_version(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        version: u64,
    ) -> StorageResult<(Vec<u8>, VersionInfo)>;

    /// Lists all available versions for a key
    fn list_versions(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<Vec<VersionInfo>>;

    /// Compare two versions and return differences
    fn diff_versions(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        v1: u64,
        v2: u64,
    ) -> StorageResult<VersionDiff<Vec<u8>>>;

    /// Sets raw byte data for a key within a namespace.
    /// Performs permission checks and resource accounting.
    /// Updates version information.
    fn set(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: Vec<u8>,
    ) -> StorageResult<()>;

    /// List keys in a namespace
    fn list_keys(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        prefix: Option<&str>,
    ) -> StorageResult<Vec<String>>;

    /// List sub-namespaces
    fn list_namespaces(
        &self,
        auth: Option<&AuthContext>,
        parent_namespace: &str,
    ) -> StorageResult<Vec<NamespaceMetadata>>;

    /// Creates a resource account for a user.
    /// Typically requires administrative privileges.
    fn create_account(
        &mut self,
        auth: Option<&AuthContext>,
        user_id: &str,
        quota_bytes: u64,
    ) -> StorageResult<()>;

    /// Creates a new namespace
    fn create_namespace(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        quota_bytes: u64,
        parent: Option<&str>,
    ) -> StorageResult<()>;

    /// Checks if the user has the required permission for an action in a namespace.
    /// This might be used internally by other methods or exposed for direct checks.
    fn check_permission(
        &self,
        auth: Option<&AuthContext>,
        action: &str,
        namespace: &str,
    ) -> StorageResult<()>;

    /// Begins a transaction.
    /// Subsequent `set` operations should be part of this transaction until commit/rollback.
    fn begin_transaction(&mut self) -> StorageResult<()>;

    /// Commits the current transaction, making changes permanent.
    fn commit_transaction(&mut self) -> StorageResult<()>;

    /// Rolls back the current transaction, discarding changes.
    fn rollback_transaction(&mut self) -> StorageResult<()>;

    /// Retrieves audit log entries, potentially filtered.
    /// Requires appropriate permissions.
    fn get_audit_log(
        &self,
        auth: Option<&AuthContext>,
        namespace: Option<&str>,
        event_type: Option<&str>,
        limit: usize,
    ) -> StorageResult<Vec<StorageEvent>>;

    /// Delete a key and its versions
    fn delete(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<()>;

    /// Get storage usage for a namespace
    fn get_usage(&self, auth: Option<&AuthContext>, namespace: &str) -> StorageResult<u64>;
}

// Convenience extension trait for serializing/deserializing values
pub trait StorageExtensions: StorageBackend {
    fn get_json<T: DeserializeOwned>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T>;
    fn set_json<T: Serialize>(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()>;
}

// Default implementation for the extension trait
impl<S: StorageBackend> StorageExtensions for S {
    fn get_json<T: DeserializeOwned>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T> {
        let bytes = self.get(auth, namespace, key)?;
        serde_json::from_slice(&bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: e.to_string(),
            }
        })
    }

    fn set_json<T: Serialize>(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()> {
        let bytes = serde_json::to_vec(value).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: e.to_string(),
            }
        })?;
        self.set(auth, namespace, key, bytes)
    }
}

// Potential trait for federated operations (optional for now)
// pub trait FederatedStorageBackend: StorageBackend {
//     fn push(&self, remote_target: &str, namespace: &str) -> StorageResult<()>;
//     fn pull(&mut self, remote_source: &str, namespace: &str) -> StorageResult<()>;
// }
