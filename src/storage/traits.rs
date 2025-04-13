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
}

// Blanket impl for all types implementing StorageBackend
impl<S: StorageBackend> StorageExtensions for S {
    fn get_identity(&self, identity_id: &str) -> StorageResult<crate::identity::Identity> {
        let key = format!("identities/{}", identity_id);
        let bytes = self.get(None, "identity", &key)?;
        serde_json::from_slice(&bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: e.to_string(),
            }
        })
    }

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

    fn get_version_json<T: DeserializeOwned>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        version: u64,
    ) -> StorageResult<Option<T>> {
        // Try to get the version, handle "not found" gracefully
        match self.get_version(auth, namespace, key, version) {
            Ok((bytes, _)) => {
                let value = serde_json::from_slice(&bytes).map_err(|e| {
                    crate::storage::errors::StorageError::SerializationError {
                        details: e.to_string(),
                    }
                })?;
                Ok(Some(value))
            }
            Err(crate::storage::errors::StorageError::KeyNotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    fn get_proposal_logic_path(&self, proposal_id: &str) -> StorageResult<String> {
        // Get the logic path from the proposal metadata
        let meta_key = format!("proposals/{}/metadata", proposal_id);
        let bytes = self.get(None, "governance", &meta_key)?;
        
        // Parse metadata to extract logic_path
        let metadata: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: format!("Failed to parse proposal metadata: {}", e),
            }
        })?;
        
        // Extract logic path from metadata
        match metadata.get("logic_path") {
            Some(path) => match path.as_str() {
                Some(path_str) => Ok(path_str.to_string()),
                None => Err(crate::storage::errors::StorageError::InvalidValue {
                    details: "logic_path is not a string".to_string(),
                }),
            },
            None => Err(crate::storage::errors::StorageError::KeyNotFound {
                key: "logic_path".to_string(),
            }),
        }
    }
    
    fn get_proposal_logic(&self, logic_path: &str) -> StorageResult<String> {
        // Get the DSL code from the specified path
        let bytes = self.get(None, "governance", logic_path)?;
        
        // Convert bytes to string
        String::from_utf8(bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: format!("Invalid UTF-8 in DSL code: {}", e),
            }
        })
    }
    
    fn save_proposal_execution_result(&mut self, proposal_id: &str, result: &str) -> StorageResult<()> {
        // Create the key for storing execution result
        let result_key = format!("proposals/{}/execution_result", proposal_id);
        
        // Save the result as a string
        self.set(None, "governance", &result_key, result.as_bytes().to_vec())
    }
    
    fn get_proposal_execution_result(&self, proposal_id: &str) -> StorageResult<String> {
        // Create the key for retrieving execution result
        let result_key = format!("proposals/{}/execution_result", proposal_id);
        
        // Try to get execution result, return error if not found
        match self.get(None, "governance", &result_key) {
            Ok(bytes) => String::from_utf8(bytes).map_err(|e| {
                crate::storage::errors::StorageError::SerializationError {
                    details: format!("Invalid UTF-8 in execution result: {}", e),
                }
            }),
            Err(e) => Err(e),
        }
    }
    
    fn get_proposal_execution_logs(&self, proposal_id: &str) -> StorageResult<String> {
        // Create the key for retrieving execution logs
        let logs_key = format!("proposals/{}/execution_logs", proposal_id);
        
        // Try to get logs, return empty string if not found
        match self.get(None, "governance", &logs_key) {
            Ok(bytes) => String::from_utf8(bytes).map_err(|e| {
                crate::storage::errors::StorageError::SerializationError {
                    details: format!("Invalid UTF-8 in execution logs: {}", e),
                }
            }),
            Err(crate::storage::errors::StorageError::KeyNotFound { .. }) => Ok(String::new()),
            Err(e) => Err(e),
        }
    }
}

/// Supertrait combining StorageBackend and StorageExtensions for use in trait objects.
pub trait Storage: StorageBackend + StorageExtensions {}

/// Blanket implementation for the Storage supertrait.
impl<T: StorageBackend + StorageExtensions> Storage for T {}
