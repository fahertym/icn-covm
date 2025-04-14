use chrono::{DateTime, Utc};
use crate::storage::auth::AuthContext;
use crate::storage::errors::StorageResult;
use crate::storage::events::StorageEvent;
use crate::storage::namespaces::NamespaceMetadata;
use crate::storage::versioning::{VersionDiff, VersionInfo};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

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

    /// Check if a key exists in a namespace
    fn contains(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<bool>;

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

// Convenience extension trait - with methods that depend on StorageBackend
pub trait StorageExtensions: StorageBackend {
    /// Retrieves an identity by ID from storage
    fn get_identity(&self, identity_id: &str) -> StorageResult<crate::identity::Identity>;

    /// Gets data as JSON from storage, deserializing it to the specified type
    fn get_json<T: DeserializeOwned>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T>;

    /// Stores data as JSON in storage
    fn set_json<T: Serialize>(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()>;

    /// Retrieves a specific version of data as JSON, deserializing it to the specified type
    fn get_version_json<T: DeserializeOwned>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        version: u64,
    ) -> StorageResult<Option<T>>;
    
    /// Gets the logic path for a proposal
    fn get_proposal_logic_path(&self, proposal_id: &str) -> StorageResult<String>;
    
    /// Gets the DSL logic code for a proposal
    fn get_proposal_logic(&self, logic_path: &str) -> StorageResult<String>;
    
    /// Saves the execution result of a proposal
    fn save_proposal_execution_result(&mut self, proposal_id: &str, result: &str) -> StorageResult<()>;
    
    /// Gets the execution result of a proposal
    fn get_proposal_execution_result(&self, proposal_id: &str) -> StorageResult<String>;
    
    /// Gets the execution logs of a proposal
    fn get_proposal_execution_logs(&self, proposal_id: &str) -> StorageResult<String>;
    
    /// Appends to the execution logs of a proposal
    fn append_proposal_execution_log(&mut self, proposal_id: &str, log_entry: &str) -> StorageResult<()>;
    
    /// Saves a versioned execution result of a proposal
    fn save_proposal_execution_result_versioned(&mut self, proposal_id: &str, result: &str, success: bool, summary: &str) -> StorageResult<u64>;
    
    /// Gets a versioned execution result of a proposal
    fn get_proposal_execution_result_versioned(&self, proposal_id: &str, version: u64) -> StorageResult<String>;
    
    /// Gets the latest execution result version for a proposal
    fn get_latest_execution_result_version(&self, proposal_id: &str) -> StorageResult<u64>;
    
    /// Gets the latest execution result for a proposal
    fn get_latest_execution_result(&self, proposal_id: &str) -> StorageResult<String>;
    
    /// Lists all execution version metadata for a proposal
    fn list_execution_versions(&self, proposal_id: &str) -> StorageResult<Vec<ExecutionVersionMeta>>;
    
    /// Gets the retry history for a proposal by parsing execution logs
    fn get_proposal_retry_history(&self, proposal_id: &str) -> StorageResult<Vec<RetryHistoryRecord>>;
}

/// Metadata about a proposal execution version
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExecutionVersionMeta {
    pub version: u64,
    pub executed_at: String,
    pub success: bool,
    pub summary: String,
}

/// Record of a proposal execution retry attempt
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RetryHistoryRecord {
    pub timestamp: String,
    pub user: String,
    pub status: String,
    pub retry_count: Option<u64>,
    pub reason: Option<String>,
}

/// Trait for async access to DSL macro operations
#[async_trait::async_trait]
pub trait AsyncStorageExtensions {
    /// Retrieves a macro definition by ID
    async fn get_macro(&self, id: &str) -> StorageResult<crate::storage::MacroDefinition>;
    
    /// Saves a macro definition
    async fn save_macro(&self, macro_def: &crate::storage::MacroDefinition) -> StorageResult<()>;
    
    /// Deletes a macro by ID
    async fn delete_macro(&self, id: &str) -> StorageResult<()>;
    
    /// Lists macros with pagination and optional sorting and filtering
    async fn list_macros(
        &self,
        page: Option<u32>,
        page_size: Option<u32>,
        sort_by: Option<String>,
        category: Option<String>,
    ) -> StorageResult<crate::api::v1::models::MacroListResponse>;
}

/// Marker trait to indicate that a type provides both synchronous and asynchronous storage operations
pub trait Storage: StorageBackend + StorageExtensions + AsyncStorageExtensions {}

/// Implement the marker trait for any type that implements both required traits
impl<T: StorageBackend + StorageExtensions + AsyncStorageExtensions> Storage for T {}

// Implement AsyncStorageExtensions for Arc<Mutex<S>> to delegate to the inner storage
#[async_trait::async_trait]
impl<S> AsyncStorageExtensions for Arc<Mutex<S>> 
where
    S: StorageBackend + StorageExtensions + AsyncStorageExtensions + Send + Sync + 'static
{
    async fn get_macro(&self, id: &str) -> StorageResult<crate::storage::MacroDefinition> {
        let storage = self.lock().await;
        storage.get_macro(id).await
    }

    async fn save_macro(&self, macro_def: &crate::storage::MacroDefinition) -> StorageResult<()> {
        let mut storage = self.lock().await;
        storage.save_macro(macro_def).await
    }

    async fn delete_macro(&self, id: &str) -> StorageResult<()> {
        let mut storage = self.lock().await;
        storage.delete_macro(id).await
    }

    async fn list_macros(
        &self,
        page: Option<u32>,
        page_size: Option<u32>,
        sort_by: Option<String>,
        category: Option<String>,
    ) -> StorageResult<crate::api::v1::models::MacroListResponse> {
        let storage = self.lock().await;
        storage.list_macros(page, page_size, sort_by, category).await
    }
}

// Implement StorageBackend for Arc<Mutex<S>> to delegate to the inner storage
impl<S> StorageBackend for Arc<Mutex<S>>
where
    S: StorageBackend + Send + Sync + 'static,
{
    fn get(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<Vec<u8>> {
        // This is a blocking operation in an async context, but it's okay for the trait impl
        // In practice, this should be used with the async wrapper functions
        let storage = futures::executor::block_on(self.lock());
        storage.get(auth, namespace, key)
    }

    fn get_versioned(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<(Vec<u8>, VersionInfo)> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_versioned(auth, namespace, key)
    }

    fn set(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: Vec<u8>,
    ) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.set(auth, namespace, key, value)
    }

    fn delete(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.delete(auth, namespace, key)
    }

    fn list_keys(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        prefix: Option<&str>,
    ) -> StorageResult<Vec<String>> {
        let storage = futures::executor::block_on(self.lock());
        storage.list_keys(auth, namespace, prefix)
    }

    fn contains(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<bool> {
        let storage = futures::executor::block_on(self.lock());
        storage.contains(auth, namespace, key)
    }

    fn check_permission(
        &self,
        auth: Option<&AuthContext>,
        action: &str,
        namespace: &str,
    ) -> StorageResult<()> {
        let storage = futures::executor::block_on(self.lock());
        storage.check_permission(auth, action, namespace)
    }

    fn begin_transaction(&mut self) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.begin_transaction()
    }

    fn commit_transaction(&mut self) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.commit_transaction()
    }

    fn rollback_transaction(&mut self) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.rollback_transaction()
    }

    fn get_audit_log(
        &self,
        auth: Option<&AuthContext>,
        namespace: Option<&str>,
        event_type: Option<&str>,
        limit: usize,
    ) -> StorageResult<Vec<crate::storage::StorageEvent>> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_audit_log(auth, namespace, event_type, limit)
    }

    fn get_version(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        version: u64,
    ) -> StorageResult<(Vec<u8>, VersionInfo)> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_version(auth, namespace, key, version)
    }

    fn list_versions(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<Vec<VersionInfo>> {
        let storage = futures::executor::block_on(self.lock());
        storage.list_versions(auth, namespace, key)
    }

    fn diff_versions(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        v1: u64,
        v2: u64,
    ) -> StorageResult<VersionDiff<Vec<u8>>> {
        let storage = futures::executor::block_on(self.lock());
        storage.diff_versions(auth, namespace, key, v1, v2)
    }

    fn create_namespace(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        quota_bytes: u64,
        parent_namespace: Option<&str>,
    ) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.create_namespace(auth, namespace, quota_bytes, parent_namespace)
    }

    fn list_namespaces(
        &self,
        auth: Option<&AuthContext>,
        parent_namespace: &str,
    ) -> StorageResult<Vec<NamespaceMetadata>> {
        let storage = futures::executor::block_on(self.lock());
        storage.list_namespaces(auth, parent_namespace)
    }

    fn get_usage(&self, auth: Option<&AuthContext>, namespace: &str) -> StorageResult<u64> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_usage(auth, namespace)
    }

    fn create_account(
        &mut self,
        auth: Option<&AuthContext>,
        user_id: &str,
        quota_bytes: u64,
    ) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.create_account(auth, user_id, quota_bytes)
    }
}

// Implement StorageExtensions for Arc<Mutex<S>> to delegate to the inner storage
impl<S> StorageExtensions for Arc<Mutex<S>>
where
    S: StorageExtensions + Send + Sync + 'static,
{
    fn get_identity(&self, identity_id: &str) -> StorageResult<crate::identity::Identity> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_identity(identity_id)
    }

    fn get_json<T: DeserializeOwned>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_json(auth, namespace, key)
    }

    fn set_json<T: Serialize>(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.set_json(auth, namespace, key, value)
    }

    fn get_version_json<T: DeserializeOwned>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        version: u64,
    ) -> StorageResult<Option<T>> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_version_json(auth, namespace, key, version)
    }

    fn get_proposal_logic_path(&self, proposal_id: &str) -> StorageResult<String> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_proposal_logic_path(proposal_id)
    }

    fn get_proposal_logic(&self, logic_path: &str) -> StorageResult<String> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_proposal_logic(logic_path)
    }

    fn save_proposal_execution_result(&mut self, proposal_id: &str, result: &str) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.save_proposal_execution_result(proposal_id, result)
    }

    fn get_proposal_execution_result(&self, proposal_id: &str) -> StorageResult<String> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_proposal_execution_result(proposal_id)
    }

    fn get_proposal_execution_logs(&self, proposal_id: &str) -> StorageResult<String> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_proposal_execution_logs(proposal_id)
    }

    fn append_proposal_execution_log(&mut self, proposal_id: &str, log_entry: &str) -> StorageResult<()> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.append_proposal_execution_log(proposal_id, log_entry)
    }

    fn save_proposal_execution_result_versioned(&mut self, proposal_id: &str, result: &str, success: bool, summary: &str) -> StorageResult<u64> {
        let mut storage = futures::executor::block_on(self.lock());
        storage.save_proposal_execution_result_versioned(proposal_id, result, success, summary)
    }

    fn get_proposal_execution_result_versioned(&self, proposal_id: &str, version: u64) -> StorageResult<String> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_proposal_execution_result_versioned(proposal_id, version)
    }

    fn get_latest_execution_result_version(&self, proposal_id: &str) -> StorageResult<u64> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_latest_execution_result_version(proposal_id)
    }

    fn get_latest_execution_result(&self, proposal_id: &str) -> StorageResult<String> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_latest_execution_result(proposal_id)
    }

    fn list_execution_versions(&self, proposal_id: &str) -> StorageResult<Vec<ExecutionVersionMeta>> {
        let storage = futures::executor::block_on(self.lock());
        storage.list_execution_versions(proposal_id)
    }

    fn get_proposal_retry_history(&self, proposal_id: &str) -> StorageResult<Vec<RetryHistoryRecord>> {
        let storage = futures::executor::block_on(self.lock());
        storage.get_proposal_retry_history(proposal_id)
    }
}

// Explicitly implement Storage for Arc<Mutex<S>> where S is Storage
// This implementation is causing conflicts, so comment it out as it's not needed
// The trait is already implemented via the blanket implementation above
// impl<S> Storage for Arc<Mutex<S>> where S: Storage + Send + Sync + 'static {}

// Implement AsyncStorageExtensions for Arc<Mutex<S>> to allow async access through the mutex
#[async_trait::async_trait]
impl<S> AsyncStorageExtensions for std::sync::Arc<std::sync::Mutex<S>>
where 
    S: AsyncStorageExtensions + Send + Sync + Clone + 'static 
{
    async fn get_macro(&self, id: &str) -> StorageResult<crate::storage::MacroDefinition> {
        // Create a clone of the inner storage to avoid holding the MutexGuard across await points
        let inner = {
            let guard = self.lock().unwrap();
            guard.clone()
        };
        
        // Now we can safely await on the cloned storage
        AsyncStorageExtensions::get_macro(&inner, id).await
    }

    async fn save_macro(&self, macro_def: &crate::storage::MacroDefinition) -> StorageResult<()> {
        // Create a clone of the inner storage to avoid holding the MutexGuard across await points
        let inner = {
            let guard = self.lock().unwrap();
            guard.clone()
        };
        
        // Now we can safely await on the cloned storage
        AsyncStorageExtensions::save_macro(&inner, macro_def).await
    }

    async fn delete_macro(&self, id: &str) -> StorageResult<()> {
        // Create a clone of the inner storage to avoid holding the MutexGuard across await points
        let inner = {
            let guard = self.lock().unwrap();
            guard.clone()
        };
        
        // Now we can safely await on the cloned storage
        AsyncStorageExtensions::delete_macro(&inner, id).await
    }

    async fn list_macros(
        &self, 
        page: Option<u32>, 
        page_size: Option<u32>, 
        sort_by: Option<String>, 
        category: Option<String>
    ) -> StorageResult<crate::api::v1::models::MacroListResponse> {
        // Create a clone of the inner storage to avoid holding the MutexGuard across await points
        let inner = {
            let guard = self.lock().unwrap();
            guard.clone()
        };
        
        // Now we can safely await on the cloned storage
        AsyncStorageExtensions::list_macros(&inner, page, page_size, sort_by, category).await
    }
}
