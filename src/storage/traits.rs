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
    
    fn append_proposal_execution_log(&mut self, proposal_id: &str, log_entry: &str) -> StorageResult<()> {
        // Create the key for execution logs
        let logs_key = format!("proposals/{}/execution_logs", proposal_id);
        
        // Get existing logs, if any
        let existing_logs = match self.get(None, "governance", &logs_key) {
            Ok(bytes) => String::from_utf8(bytes).map_err(|e| {
                crate::storage::errors::StorageError::SerializationError {
                    details: format!("Invalid UTF-8 in execution logs: {}", e),
                }
            })?,
            Err(crate::storage::errors::StorageError::KeyNotFound { .. }) => String::new(),
            Err(e) => return Err(e),
        };
        
        // Append the new log entry to existing logs
        let updated_logs = if existing_logs.is_empty() {
            log_entry.to_string()
        } else {
            format!("{}\n{}", existing_logs, log_entry)
        };
        
        // Save the updated logs
        self.set(None, "governance", &logs_key, updated_logs.as_bytes().to_vec())
    }
    
    fn save_proposal_execution_result_versioned(&mut self, proposal_id: &str, result: &str, success: bool, summary: &str) -> StorageResult<u64> {
        // Get the latest version and increment it
        let next_version = match self.get_latest_execution_result_version(proposal_id) {
            Ok(version) => version + 1,
            Err(_) => 1, // Start with version 1 if no versions exist
        };
        
        // Create the key for storing versioned execution result
        let result_key = format!("proposals/{}/execution_results/{}", proposal_id, next_version);
        
        // Save the result as a string
        self.set(None, "governance", &result_key, result.as_bytes().to_vec())?;
        
        // Save metadata for this version
        let meta = ExecutionVersionMeta {
            version: next_version,
            executed_at: crate::storage::now().to_string(),
            success,
            summary: summary.to_string(),
        };
        
        let meta_key = format!("proposals/{}/execution_results/{}/meta", proposal_id, next_version);
        let meta_bytes = serde_json::to_vec(&meta).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                details: format!("Failed to serialize execution metadata: {}", e),
            }
        })?;
        
        self.set(None, "governance", &meta_key, meta_bytes)?;
        
        // Update the latest version pointer
        let latest_key = format!("proposals/{}/execution_results/latest", proposal_id);
        let version_bytes = next_version.to_string().into_bytes();
        self.set(None, "governance", &latest_key, version_bytes)?;
        
        Ok(next_version)
    }
    
    fn get_proposal_execution_result_versioned(&self, proposal_id: &str, version: u64) -> StorageResult<String> {
        // Create the key for retrieving versioned execution result
        let result_key = format!("proposals/{}/execution_results/{}", proposal_id, version);
        
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
    
    fn get_latest_execution_result_version(&self, proposal_id: &str) -> StorageResult<u64> {
        // Create the key for retrieving the latest version pointer
        let latest_key = format!("proposals/{}/execution_results/latest", proposal_id);
        
        // Try to get the latest version
        match self.get(None, "governance", &latest_key) {
            Ok(bytes) => {
                let version_str = String::from_utf8(bytes).map_err(|e| {
                    crate::storage::errors::StorageError::SerializationError {
                        details: format!("Invalid UTF-8 in version pointer: {}", e),
                    }
                })?;
                
                version_str.parse::<u64>().map_err(|e| {
                    crate::storage::errors::StorageError::SerializationError {
                        details: format!("Invalid version number: {}", e),
                    }
                })
            },
            // If the latest pointer doesn't exist, list all versions and find the max
            Err(crate::storage::errors::StorageError::NotFound { .. }) => {
                let prefix = format!("proposals/{}/execution_results/", proposal_id);
                let keys = self.list_keys(None, "governance", Some(&prefix))?;
                
                // Find max version from keys
                let mut max_version = 0;
                for key in keys {
                    if let Some(version_str) = key.strip_prefix(&prefix) {
                        if !version_str.contains('/') { // Skip metadata keys with /
                            if let Ok(version) = version_str.parse::<u64>() {
                                if version > max_version {
                                    max_version = version;
                                }
                            }
                        }
                    }
                }
                
                if max_version == 0 {
                    Err(crate::storage::errors::StorageError::NotFound {
                        key: format!("No execution results for proposal {}", proposal_id),
                    })
                } else {
                    Ok(max_version)
                }
            },
            Err(e) => Err(e),
        }
    }
    
    fn get_latest_execution_result(&self, proposal_id: &str) -> StorageResult<String> {
        // Get the latest version
        let latest_version = self.get_latest_execution_result_version(proposal_id)?;
        
        // Get the result for that version
        self.get_proposal_execution_result_versioned(proposal_id, latest_version)
    }
    
    fn list_execution_versions(&self, proposal_id: &str) -> StorageResult<Vec<ExecutionVersionMeta>> {
        let prefix = format!("proposals/{}/execution_results/", proposal_id);
        let keys = self.list_keys(None, "governance", Some(&prefix))?;
        
        let mut versions = Vec::new();
        
        // Filter for metadata keys and collect version info
        for key in keys {
            if key.ends_with("/meta") {
                match self.get(None, "governance", &key) {
                    Ok(bytes) => {
                        match serde_json::from_slice::<ExecutionVersionMeta>(&bytes) {
                            Ok(meta) => versions.push(meta),
                            Err(_) => continue, // Skip invalid metadata
                        }
                    },
                    Err(_) => continue, // Skip if can't read metadata
                }
            }
        }
        
        // Sort by version (descending)
        versions.sort_by(|a, b| b.version.cmp(&a.version));
        
        Ok(versions)
    }
    
    fn get_proposal_retry_history(&self, proposal_id: &str) -> StorageResult<Vec<RetryHistoryRecord>> {
        // Get the execution logs
        let logs = self.get_proposal_execution_logs(proposal_id)?;
        let mut records = Vec::new();

        // Parse each log line that contains RETRY information
        for line in logs.lines() {
            // Retry log entries follow this format:
            // [timestamp] RETRY by user:username | status: success/failed | retry_count: N
            // or for failures with reason:
            // [timestamp] RETRY by user:username | status: failed | reason: reason_text
            
            if line.contains("RETRY by user:") {
                // Extract timestamp, which is enclosed in square brackets
                let timestamp = if let Some(end_idx) = line.find(']') {
                    if line.starts_with('[') {
                        line[1..end_idx].to_string()
                    } else {
                        continue; // Invalid format, skip this line
                    }
                } else {
                    continue; // No closing bracket, skip this line
                };
                
                // Extract user
                let user_start = line.find("user:").map(|idx| idx + 5).unwrap_or(0);
                let user_end = line[user_start..].find(" |").map(|idx| user_start + idx).unwrap_or(line.len());
                let user = if user_start > 0 && user_end > user_start {
                    line[user_start..user_end].to_string()
                } else {
                    "unknown".to_string()
                };
                
                // Extract status
                let status = if line.contains("status: success") {
                    "success".to_string()
                } else if line.contains("status: failed") {
                    "failed".to_string()
                } else {
                    "unknown".to_string()
                };
                
                // Extract retry count if present
                let retry_count = if let Some(count_idx) = line.find("retry_count: ") {
                    let count_start = count_idx + 13; // "retry_count: ".len()
                    let count_end = line[count_start..].find(" |").map(|idx| count_start + idx).unwrap_or(line.len());
                    line[count_start..count_end].parse::<u64>().ok()
                } else {
                    None
                };
                
                // Extract reason for failures
                let reason = if status == "failed" && line.contains("reason: ") {
                    let reason_start = line.find("reason: ").map(|idx| idx + 8).unwrap_or(0);
                    if reason_start > 0 {
                        Some(line[reason_start..].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Add to records
                records.push(RetryHistoryRecord {
                    timestamp,
                    user,
                    status,
                    retry_count,
                    reason,
                });
            }
        }
        
        // Sort by timestamp descending (newest first)
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(records)
    }
}

/// Supertrait combining StorageBackend and StorageExtensions for use in trait objects.
pub trait Storage: StorageBackend + StorageExtensions {}

/// Blanket implementation for the Storage supertrait.
impl<T: StorageBackend + StorageExtensions> Storage for T {}
