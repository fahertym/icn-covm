use std::path::{Path, PathBuf};
use std::fs::{self, File, create_dir_all, OpenOptions};
use std::io::{self, Read, Write, BufRead, BufReader};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::storage::traits::StorageBackend;
use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::versioning::VersionInfo;
use crate::storage::namespaces::NamespaceMetadata;
use crate::storage::utils::{now, Timestamp};
use crate::storage::events::StorageEvent;

/// Represents a file-based persistent storage implementation.
/// 
/// The FileStorage organizes data in a hierarchical directory structure:
/// - namespaces/ - Contains all namespaced data
///   - {namespace}/ - Namespace directories (e.g., governance/proposals)
///     - keys/ - Contains all keys in this namespace
///       - {key}/ - Directory for each key
///         - v{version}.data - Versioned data files
///         - metadata.json - Version and key metadata
///     - namespace_metadata.json - Namespace configuration
/// - accounts/ - User account information
/// - audit_logs/ - Append-only logs of all operations
/// - transactions/ - Transaction logs and rollback information
pub struct FileStorage {
    /// Root path for all storage
    root_path: PathBuf,
    /// Active transactions
    transactions: Vec<Vec<TransactionOp>>,
    /// In-memory cache of namespace metadata (for performance)
    namespace_cache: HashMap<String, NamespaceMetadata>,
    /// In-memory cache of account data (for performance)
    account_cache: HashMap<String, ResourceAccount>,
}

/// Represents a user's resource account for storage quota management
#[derive(Clone, Serialize, Deserialize)]
struct ResourceAccount {
    user_id: String,
    quota_bytes: u64,
    used_bytes: u64,
    #[serde(with = "timestamp_serde")]
    created_at: Timestamp,
    #[serde(with = "timestamp_serde")]
    last_updated: Timestamp,
}

/// Serialization helpers for Timestamp
mod timestamp_serde {
    use super::*;
    use serde::{Serializer, Deserializer};
    
    pub fn serialize<S>(timestamp: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(*timestamp)
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer)
    }
}

/// Key metadata stored alongside the versioned data files
#[derive(Serialize, Deserialize)]
struct KeyMetadata {
    key: String,
    created_by: String,
    #[serde(with = "timestamp_serde")]
    created_at: Timestamp,
    versions: Vec<VersionInfo>,
}

/// Represents a transaction operation for rollback support
#[derive(Clone, Serialize, Deserialize)]
enum TransactionOp {
    Set {
        namespace: String,
        key: String,
        previous_data: Option<Vec<u8>>,
        prev_version: Option<VersionInfo>,
    },
    Delete {
        namespace: String,
        key: String,
        previous_data: Vec<u8>,
        previous_version: u64,
    },
    CreateNamespace {
        namespace: String,
    },
    DeleteNamespace {
        namespace: String,
        metadata: NamespaceMetadata,
    },
}

/// Represents an audit log entry
#[derive(Serialize, Deserialize)]
struct AuditLogEntry {
    #[serde(with = "timestamp_serde")]
    timestamp: Timestamp,
    user_id: String,
    action: String,
    namespace: String,
    key: Option<String>,
    details: String,
}

impl FileStorage {
    /// Creates a new FileStorage with the specified root directory
    pub fn new<P: AsRef<Path>>(root_path: P) -> StorageResult<Self> {
        let root = root_path.as_ref().to_path_buf();
        
        // Create the basic directory structure if it doesn't exist
        create_dir_all(root.join("namespaces"))?;
        create_dir_all(root.join("accounts"))?;
        create_dir_all(root.join("audit_logs"))?;
        create_dir_all(root.join("transactions"))?;
        
        // Initialize an empty storage
        let mut storage = FileStorage {
            root_path: root,
            transactions: Vec::new(),
            namespace_cache: HashMap::new(),
            account_cache: HashMap::new(),
        };
        
        // Load namespace metadata into cache
        storage.load_namespace_cache()?;
        
        // Load account data into cache
        storage.load_account_cache()?;
        
        Ok(storage)
    }
    
    /// Loads namespace metadata into the in-memory cache
    fn load_namespace_cache(&mut self) -> StorageResult<()> {
        self.namespace_cache.clear();
        
        // Start with the root namespaces directory
        let namespaces_dir = self.root_path.join("namespaces");
        
        // Walk the directory tree to find all namespace_metadata.json files
        self.load_namespaces_recursive(&namespaces_dir, None)?;
        
        Ok(())
    }
    
    /// Recursively loads namespace metadata from all subdirectories
    fn load_namespaces_recursive(&mut self, dir: &Path, parent: Option<&str>) -> StorageResult<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        
        // Check if this directory has a namespace_metadata.json file
        let metadata_path = dir.join("namespace_metadata.json");
        if metadata_path.exists() {
            let metadata_str = fs::read_to_string(metadata_path)?;
            let metadata: NamespaceMetadata = serde_json::from_str(&metadata_str)
                .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
            
            // Add to cache
            self.namespace_cache.insert(metadata.path.clone(), metadata);
        }
        
        // Recursively check subdirectories, but skip the 'keys' directory
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() && path.file_name().unwrap_or_default() != "keys" {
                self.load_namespaces_recursive(&path, parent)?;
            }
        }
        
        Ok(())
    }
    
    /// Loads user account data into the in-memory cache
    fn load_account_cache(&mut self) -> StorageResult<()> {
        self.account_cache.clear();
        
        let accounts_dir = self.root_path.join("accounts");
        if !accounts_dir.exists() {
            return Ok(());
        }
        
        for entry in fs::read_dir(accounts_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().unwrap_or_default() == "json" {
                let account_str = fs::read_to_string(&path)?;
                let account: ResourceAccount = serde_json::from_str(&account_str)
                    .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
                
                // Add to cache
                self.account_cache.insert(account.user_id.clone(), account);
            }
        }
        
        Ok(())
    }
    
    /// Gets the path to a namespace directory
    fn namespace_path(&self, namespace: &str) -> PathBuf {
        self.root_path.join("namespaces").join(namespace)
    }
    
    /// Gets the path to a key's directory within a namespace
    fn key_dir_path(&self, namespace: &str, key: &str) -> PathBuf {
        self.namespace_path(namespace).join("keys").join(key)
    }
    
    /// Gets the path to a specific version of a key's data
    fn version_path(&self, namespace: &str, key: &str, version: u64) -> PathBuf {
        self.key_dir_path(namespace, key).join(format!("v{}.data", version))
    }
    
    /// Gets the path to a key's metadata file
    fn metadata_path(&self, namespace: &str, key: &str) -> PathBuf {
        self.key_dir_path(namespace, key).join("metadata.json")
    }
    
    /// Gets the path to a namespace's metadata file
    fn namespace_metadata_path(&self, namespace: &str) -> PathBuf {
        self.namespace_path(namespace).join("namespace_metadata.json")
    }
    
    /// Reads key metadata from disk
    fn read_key_metadata(&self, namespace: &str, key: &str) -> StorageResult<KeyMetadata> {
        let path = self.metadata_path(namespace, key);
        
        if !path.exists() {
            return Err(StorageError::NotFound {
                key: format!("{}:{}", namespace, key),
            });
        }
        
        let metadata_str = fs::read_to_string(path)?;
        let metadata: KeyMetadata = serde_json::from_str(&metadata_str)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        Ok(metadata)
    }
    
    /// Writes key metadata to disk
    fn write_key_metadata(&self, namespace: &str, key: &str, metadata: &KeyMetadata) -> StorageResult<()> {
        let path = self.metadata_path(namespace, key);
        
        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        
        let metadata_str = serde_json::to_string(metadata)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        fs::write(path, metadata_str)?;
        
        Ok(())
    }
    
    /// Writes data to a specific version file
    fn write_version_data(&self, namespace: &str, key: &str, version: u64, data: &[u8]) -> StorageResult<()> {
        let path = self.version_path(namespace, key, version);
        
        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        
        fs::write(path, data)?;
        
        Ok(())
    }
    
    /// Reads data from a specific version file
    fn read_version_data(&self, namespace: &str, key: &str, version: u64) -> StorageResult<Vec<u8>> {
        let path = self.version_path(namespace, key, version);
        
        if !path.exists() {
            return Err(StorageError::NotFound {
                key: format!("{}:{} (version {})", namespace, key, version),
            });
        }
        
        let data = fs::read(path)?;
        
        Ok(data)
    }
    
    /// Writes a namespace metadata file
    fn write_namespace_metadata(&self, metadata: &NamespaceMetadata) -> StorageResult<()> {
        let path = self.namespace_metadata_path(&metadata.path);
        
        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        
        let metadata_str = serde_json::to_string(metadata)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        fs::write(path, metadata_str)?;
        
        Ok(())
    }
    
    /// Records an audit log entry
    fn record_audit_log(&self, auth: &AuthContext, action: &str, namespace: &str, key: Option<&str>, details: &str) -> StorageResult<()> {
        let now = now();
        let date = chrono::NaiveDateTime::from_timestamp_opt(now as i64, 0)
            .ok_or_else(|| StorageError::TransactionError { details: "Invalid timestamp".to_string() })?
            .format("%Y%m%d").to_string();
        
        let log_path = self.root_path.join("audit_logs").join(format!("log_{}.jsonl", date));
        
        let log_entry = AuditLogEntry {
            timestamp: now,
            user_id: auth.user_id.clone(),
            action: action.to_string(),
            namespace: namespace.to_string(),
            key: key.map(String::from),
            details: details.to_string(),
        };
        
        let log_str = serde_json::to_string(&log_entry)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        // Append to log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        
        writeln!(file, "{}", log_str)?;
        
        Ok(())
    }
    
    /// Checks if the namespace exists
    fn namespace_exists(&self, namespace: &str) -> bool {
        self.namespace_cache.contains_key(namespace) || self.namespace_path(namespace).exists()
    }
    
    /// Records an operation for potential rollback
    fn record_for_rollback(&mut self, op: TransactionOp) -> StorageResult<()> {
        if let Some(tx) = self.transactions.last_mut() {
            tx.push(op);
            Ok(())
        } else {
            // No active transaction, nothing to record
            Ok(())
        }
    }
    
    /// Internal permission logic reused by get/set/etc.
    fn check_permission_internal(&self, auth: &AuthContext, action: &str, namespace: &str) -> StorageResult<()> {
        // Global admin bypasses namespace checks
        if auth.has_role("global", "admin") {
            return Ok(());
        }

        // Check namespace admin
        if auth.has_role(namespace, "admin") {
            return Ok(());
        }

        // Role-based checks
        let required_role = match action {
            "read" => vec!["reader", "writer"], // Readers or writers can read
            "write" => vec!["writer"],          // Only writers can write
            // Add other actions like "delete", "administer"?
            _ => return Err(StorageError::PermissionDenied { 
                user_id: auth.user_id.clone(),
                action: format!("unknown action: {}", action),
                key: namespace.to_string(),
            })
        };

        if required_role.iter().any(|role| auth.has_role(namespace, role)) {
            Ok(())
        } else {
            Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: action.to_string(),
                key: namespace.to_string(),
            })
        }
    }
}

impl StorageBackend for FileStorage {
    fn get(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<Vec<u8>> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;
        
        // Check if the namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("Namespace not found: {}", namespace),
            });
        }
        
        // Try to read the key's metadata to get the latest version
        let metadata = self.read_key_metadata(namespace, key)?;
        
        // Get the latest version
        let latest_version = metadata.versions.last().ok_or_else(|| StorageError::NotFound {
            key: format!("{}:{} (no versions)", namespace, key),
        })?;
        
        // Read the data for this version
        let data = self.read_version_data(namespace, key, latest_version.version)?;
        
        // Record audit log
        self.record_audit_log(auth, "read", namespace, Some(key), &format!("Read version {}", latest_version.version))?;
        
        Ok(data)
    }
    
    fn set(&mut self, auth: &AuthContext, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()> {
        // Check permissions
        self.check_permission(auth, "write", namespace)?;
        
        // Check if namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("{}:{}", namespace, key),
            });
        }
        
        // Create the key directory if it doesn't exist
        let key_dir = self.key_dir_path(namespace, key);
        create_dir_all(&key_dir)?;
        
        // Check if metadata exists to determine if this is an update or new key
        let metadata_path = self.metadata_path(namespace, key);
        let key_metadata_exists = metadata_path.exists();
        
        // Get existing data for rollback and resource accounting
        let existing_data = if key_metadata_exists {
            match self.get(auth, namespace, key) {
                Ok(data) => Some(data),
                Err(_) => None,
            }
        } else {
            None
        };
        
        let existing_size = existing_data.as_ref().map(|d| d.len() as u64).unwrap_or(0);
        let value_size = value.len() as u64;
        
        // Resource accounting
        if value_size > existing_size {
            let additional_bytes = value_size - existing_size;
            
            // Get the user's account
            if let Some(account) = self.account_cache.get_mut(&auth.user_id) {
                if account.used_bytes + additional_bytes > account.quota_bytes {
                    return Err(StorageError::QuotaExceeded {
                        account_id: auth.user_id.clone(),
                        requested: additional_bytes,
                        available: account.quota_bytes.saturating_sub(account.used_bytes),
                    });
                }
                
                // Update the account usage
                account.used_bytes += additional_bytes;
                
                // Save the updated account
                let account_path = self.root_path.join("accounts").join(format!("{}.json", auth.user_id));
                let account_str = serde_json::to_string(account)
                    .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
                fs::write(account_path, account_str)?;
            }
        }
        
        // Find current version info for rollback purposes
        let current_version_info = if key_metadata_exists {
            let metadata = self.read_key_metadata(namespace, key)?;
            metadata.versions.last().cloned()
        } else {
            None
        };
        
        // Record operation for transaction rollback if needed
        if !self.transactions.is_empty() {
            self.record_for_rollback(TransactionOp::Set {
                namespace: namespace.to_string(),
                key: key.to_string(),
                previous_data: existing_data,
                prev_version: current_version_info,
            })?;
        }
        
        // Get the new version number
        let version_info = if key_metadata_exists {
            let mut metadata = self.read_key_metadata(namespace, key)?;
            let latest_version = metadata.versions.last()
                .map(|v| v.version)
                .unwrap_or(0);
            
            let new_version = latest_version + 1;
            let version_info = VersionInfo {
                version: new_version,
                created_by: auth.user_id.clone(),
                timestamp: now(),
                prev_version: metadata.versions.last().cloned().map(Box::new),
            };
            
            metadata.versions.push(version_info.clone());
            self.write_key_metadata(namespace, key, &metadata)?;
            
            version_info
        } else {
            // First version
            let version_info = VersionInfo {
                version: 1,
                created_by: auth.user_id.clone(),
                timestamp: now(),
                prev_version: None,
            };
            
            let metadata = KeyMetadata {
                key: key.to_string(),
                created_by: auth.user_id.clone(),
                created_at: now(),
                versions: vec![version_info.clone()],
            };
            
            self.write_key_metadata(namespace, key, &metadata)?;
            
            version_info
        };
        
        // Write the data file
        self.write_version_data(namespace, key, version_info.version, &value)?;
        
        // Record to audit log
        self.record_audit_log(
            auth,
            "write",
            namespace,
            Some(key),
            &format!("Set v{} ({} bytes)", version_info.version, value_size),
        )?;
        
        Ok(())
    }
    
    fn delete(&mut self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<()> {
        // Check write permission
        self.check_permission(auth, "write", namespace)?;
        
        // Check if the namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("Namespace not found: {}", namespace),
            });
        }
        
        // Read metadata to get version info
        let metadata = self.read_key_metadata(namespace, key)?;
        
        // Get the latest version for rollback
        let latest_version = metadata.versions.last().ok_or_else(|| StorageError::NotFound {
            key: format!("{}:{} (no versions)", namespace, key),
        })?;
        
        // Read current data for potential rollback
        let previous_data = self.read_version_data(namespace, key, latest_version.version)?;
        
        // Record for potential rollback if in a transaction
        self.record_for_rollback(TransactionOp::Delete {
            namespace: namespace.to_string(),
            key: key.to_string(),
            previous_data,
            previous_version: latest_version.version,
        })?;
        
        // Delete the key directory and all its contents
        let key_dir = self.key_dir_path(namespace, key);
        fs::remove_dir_all(key_dir)?;
        
        // Record audit log
        self.record_audit_log(auth, "delete", namespace, Some(key), 
            &format!("Deleted key with {} versions", metadata.versions.len()))?;
        
        Ok(())
    }
    
    fn list_keys(&self, auth: &AuthContext, namespace: &str, prefix: Option<&str>) -> StorageResult<Vec<String>> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;
        
        // Check if the namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("Namespace not found: {}", namespace),
            });
        }
        
        // Get the path to the keys directory
        let keys_dir = self.namespace_path(namespace).join("keys");
        
        // If the keys directory doesn't exist, return empty list
        if !keys_dir.exists() {
            return Ok(Vec::new());
        }
        
        // Collect all key directories
        let mut keys = Vec::new();
        for entry in fs::read_dir(keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(key_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(prefix_str) = prefix {
                        if key_name.starts_with(prefix_str) {
                            keys.push(key_name.to_string());
                        }
                    } else {
                        keys.push(key_name.to_string());
                    }
                }
            }
        }
        
        // Record audit log
        self.record_audit_log(auth, "list_keys", namespace, None, 
            &format!("Listed keys (found {})", keys.len()))?;
        
        Ok(keys)
    }
    
    fn begin_transaction(&mut self) -> StorageResult<()> {
        self.transactions.push(Vec::new());
        
        Ok(())
    }
    
    fn commit_transaction(&mut self) -> StorageResult<()> {
        if self.transactions.pop().is_none() {
            return Err(StorageError::TransactionError {
                details: "No active transaction to commit".to_string(),
            });
        }
        
        Ok(())
    }
    
    fn rollback_transaction(&mut self) -> StorageResult<()> {
        let transaction = match self.transactions.pop() {
            Some(txn) => txn,
            None => return Err(StorageError::TransactionError { 
                details: "No active transaction to rollback".to_string()
            }),
        };
        
        // Process operations in reverse order
        for op in transaction.into_iter().rev() {
            match op {
                TransactionOp::Set { namespace, key, previous_data, prev_version } => {
                    match (previous_data, prev_version) {
                        (Some(data), Some(version)) => {
                            // Key existed before, restore previous version
                            self.write_version_data(&namespace, &key, version.version, &data)?;
                            
                            let mut metadata = self.read_key_metadata(&namespace, &key)?;
                            // Pop off any versions greater than the one we're restoring
                            while metadata.versions.last().map_or(false, |v| v.version > version.version) {
                                metadata.versions.pop();
                            }
                            self.write_key_metadata(&namespace, &key, &metadata)?;
                        },
                        (None, _) => {
                            // Key didn't exist before, remove it
                            let key_dir = self.key_dir_path(&namespace, &key);
                            if key_dir.exists() {
                                fs::remove_dir_all(key_dir)?;
                            }
                        },
                        _ => {
                            // Shouldn't happen, but for completeness...
                            return Err(StorageError::TransactionError { 
                                details: "Inconsistent transaction data".to_string()
                            });
                        }
                    }
                },
                TransactionOp::Delete { namespace, key, previous_data, previous_version } => {
                    // Restore the deleted key
                    self.write_version_data(&namespace, &key, previous_version, &previous_data)?;
                    
                    // Recreate metadata if needed
                    if !self.metadata_path(&namespace, &key).exists() {
                        let metadata = KeyMetadata {
                            key: key.clone(),
                            created_by: "SYSTEM_ROLLBACK".to_string(),
                            created_at: now(),
                            versions: vec![VersionInfo {
                                version: previous_version,
                                created_by: "SYSTEM_ROLLBACK".to_string(),
                                timestamp: now(),
                                prev_version: None,
                            }],
                        };
                        self.write_key_metadata(&namespace, &key, &metadata)?;
                    }
                },
                TransactionOp::CreateNamespace { namespace } => {
                    // Remove the created namespace
                    let namespace_dir = self.namespace_path(&namespace);
                    if namespace_dir.exists() {
                        fs::remove_dir_all(namespace_dir)?;
                    }
                    
                    // Remove from cache
                    self.namespace_cache.remove(&namespace);
                },
                TransactionOp::DeleteNamespace { namespace, metadata } => {
                    // Restore the deleted namespace
                    self.namespace_cache.insert(namespace.clone(), metadata.clone());
                    
                    // Recreate the namespace directory structure
                    let namespace_dir = self.namespace_path(&namespace);
                    create_dir_all(&namespace_dir)?;
                    create_dir_all(namespace_dir.join("keys"))?;
                    
                    // Write the metadata file
                    let metadata_path = self.namespace_metadata_path(&namespace);
                    let metadata_str = serde_json::to_string(&metadata)
                        .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
                    fs::write(metadata_path, metadata_str)?;
                },
            }
        }
        
        Ok(())
    }
    
    fn get_version(&self, auth: &AuthContext, namespace: &str, key: &str, version: u64) -> StorageResult<(Vec<u8>, VersionInfo)> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;
        
        // Check if the namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("Namespace not found: {}", namespace),
            });
        }
        
        // Read the key's metadata
        let metadata = self.read_key_metadata(namespace, key)?;
        
        // Find the requested version
        let version_info = metadata.versions.iter()
            .find(|v| v.version == version)
            .ok_or_else(|| StorageError::NotFound {
                key: format!("{}:{} (version {})", namespace, key, version),
            })?;
        
        // Read the data for this version
        let data = self.read_version_data(namespace, key, version)?;
        
        // Record audit log
        self.record_audit_log(auth, "read_version", namespace, Some(key), 
            &format!("Read version {}", version))?;
        
        Ok((data, version_info.clone()))
    }
    
    fn list_versions(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<Vec<VersionInfo>> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;
        
        // Check if the namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("Namespace not found: {}", namespace),
            });
        }
        
        // Read the key's metadata
        let metadata = self.read_key_metadata(namespace, key)?;
        
        // Record audit log
        self.record_audit_log(auth, "list_versions", namespace, Some(key), 
            &format!("Listed versions (found {})", metadata.versions.len()))?;
        
        Ok(metadata.versions)
    }
    
    fn diff_versions(&self, auth: &AuthContext, namespace: &str, key: &str, v1: u64, v2: u64) -> StorageResult<crate::storage::versioning::VersionDiff<Vec<u8>>> {
        // Check permissions
        self.check_permission(auth, "read", namespace)?;
        
        // Check if namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("{}:{}", namespace, key),
            });
        }
        
        // Read metadata to get version info
        let metadata = self.read_key_metadata(namespace, key)?;
        
        // Find version info for v1
        let v1_info = metadata.versions.iter()
            .find(|v| v.version == v1)
            .ok_or_else(|| StorageError::NotFound {
                key: format!("{}:{}:v{}", namespace, key, v1),
            })?;
        
        // Find version info for v2
        let v2_info = metadata.versions.iter()
            .find(|v| v.version == v2)
            .ok_or_else(|| StorageError::NotFound {
                key: format!("{}:{}:v{}", namespace, key, v2),
            })?;
        
        // Read data for both versions
        let v1_data = self.read_version_data(namespace, key, v1)?;
        let v2_data = self.read_version_data(namespace, key, v2)?;
        
        // Record the audit event
        self.record_audit_log(
            auth,
            "diff",
            namespace,
            Some(key),
            &format!("Diff between v{} and v{}", v1, v2),
        )?;
        
        // For now, just a simple comparison of data size differences
        // In a real implementation, you might want to use a diff algorithm
        use crate::storage::versioning::{VersionDiff, DiffChange};
        let diff = VersionDiff {
            old_version: v1,
            new_version: v2,
            created_by: auth.user_id.clone(),
            timestamp: now(),
            changes: vec![
                DiffChange::ValueChanged {
                    path: key.to_string(),
                    old_value: v1_data,
                    new_value: v2_data,
                }
            ],
        };
        
        Ok(diff)
    }
    
    fn create_namespace(&mut self, auth: &AuthContext, namespace: &str, quota_bytes: u64, parent: Option<&str>) -> StorageResult<()> {
        // Check if user has admin permission on global or parent namespace
        let can_create = auth.has_role("global", "admin") || 
            parent.map_or(false, |p| auth.has_role(p, "admin"));
            
        if !can_create {
            return Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: "create_namespace".to_string(),
                key: namespace.to_string(),
            });
        }
        
        // Check if parent exists when specified
        if let Some(parent_ns) = parent {
            if !self.namespace_exists(parent_ns) {
                return Err(StorageError::NotFound {
                    key: format!("Parent namespace not found: {}", parent_ns),
                });
            }
        }
        
        // Check if namespace already exists
        if self.namespace_exists(namespace) {
            return Err(StorageError::TransactionError {
                details: format!("Namespace already exists: {}", namespace),
            });
        }
        
        // Create the namespace directories
        let namespace_dir = self.namespace_path(namespace);
        create_dir_all(&namespace_dir)?;
        create_dir_all(namespace_dir.join("keys"))?;
        
        // Create namespace metadata
        let metadata = NamespaceMetadata {
            path: namespace.to_string(),
            owner: auth.user_id.clone(),
            quota_bytes,
            used_bytes: 0,
            parent: parent.map(String::from),
            attributes: std::collections::HashMap::new(),
        };
        
        // Write metadata file
        self.write_namespace_metadata(&metadata)?;
        
        // Add to cache
        self.namespace_cache.insert(namespace.to_string(), metadata);
        
        // Record for potential rollback
        self.record_for_rollback(TransactionOp::CreateNamespace {
            namespace: namespace.to_string(),
        })?;
        
        // Record audit log
        self.record_audit_log(auth, "create_namespace", namespace, None,
            &format!("Created namespace with quota {} bytes", quota_bytes))?;
        
        Ok(())
    }
    
    fn list_namespaces(&self, auth: &AuthContext, parent_namespace: &str) -> StorageResult<Vec<crate::storage::namespaces::NamespaceMetadata>> {
        // Check if the parent namespace exists
        if !parent_namespace.is_empty() && !self.namespace_exists(parent_namespace) {
            return Err(StorageError::NotFound {
                key: format!("Parent namespace not found: {}", parent_namespace),
            });
        }
        
        // Collect namespaces that match the parent prefix
        let mut namespaces = Vec::new();
        
        for (path, metadata) in &self.namespace_cache {
            // Skip if this is not a child of the parent namespace
            if !parent_namespace.is_empty() {
                if metadata.parent.as_deref() != Some(parent_namespace) {
                    continue;
                }
            }
            
            // Check permission - user must have at least reader role
            if auth.has_role(path, "reader") || 
               auth.has_role(path, "writer") || 
               auth.has_role(path, "admin") ||
               auth.has_role("global", "admin") {
                namespaces.push(metadata.clone());
            }
        }
        
        // Record audit log
        self.record_audit_log(auth, "list_namespaces", parent_namespace, None,
            &format!("Listed namespaces (found {})", namespaces.len()))?;
        
        Ok(namespaces)
    }
    
    fn get_usage(&self, auth: &AuthContext, namespace: &str) -> StorageResult<u64> {
        // Check permission
        self.check_permission(auth, "read", namespace)?;
        
        // Check if the namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("Namespace not found: {}", namespace),
            });
        }
        
        // Get the namespace from cache
        if let Some(metadata) = self.namespace_cache.get(namespace) {
            return Ok(metadata.used_bytes);
        }
        
        // If not in cache, calculate size from disk
        let keys_dir = self.namespace_path(namespace).join("keys");
        if !keys_dir.exists() {
            return Ok(0);
        }
        
        let mut total_size = 0;
        
        // Walk the directory structure to calculate size
        for entry in fs::read_dir(keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // This is a key directory, calculate size of all version files
                for file_entry in fs::read_dir(path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();
                    
                    if file_path.is_file() && file_path.extension().unwrap_or_default() == "data" {
                        total_size += file_entry.metadata()?.len();
                    }
                }
            }
        }
        
        Ok(total_size)
    }
    
    fn create_account(&mut self, auth: &AuthContext, user_id: &str, quota_bytes: u64) -> StorageResult<()> {
        // Check if user has admin permission
        if !auth.has_role("global", "admin") {
            return Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: "create_account".to_string(),
                key: user_id.to_string(),
            });
        }
        
        // Check if account already exists
        if self.account_cache.contains_key(user_id) {
            return Err(StorageError::TransactionError {
                details: format!("Account already exists: {}", user_id),
            });
        }
        
        // Create account
        let account = ResourceAccount {
            user_id: user_id.to_string(),
            quota_bytes,
            used_bytes: 0,
            created_at: now(),
            last_updated: now(),
        };
        
        // Write to file
        let account_path = self.root_path.join("accounts").join(format!("{}.json", user_id));
        let account_str = serde_json::to_string(&account)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        fs::write(account_path, account_str)?;
        
        // Add to cache
        self.account_cache.insert(user_id.to_string(), account);
        
        // Record audit log
        self.record_audit_log(auth, "create_account", "global", Some(user_id),
            &format!("Created account with quota {} bytes", quota_bytes))?;
        
        Ok(())
    }
    
    fn get_audit_log(&self, auth: &AuthContext, namespace: Option<&str>, event_type: Option<&str>, limit: usize) -> StorageResult<Vec<StorageEvent>> {
        // Check permissions - only admins can access audit logs
        if !auth.has_role("global", "admin") {
            return Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: "get_audit_log".to_string(),
                key: "audit_logs".to_string(),
            });
        }
        
        let mut events = Vec::new();
        let audit_dir = self.root_path.join("audit_logs");
        
        // Filter logs by namespace if specified
        let target_dir = if let Some(ns) = namespace {
            audit_dir.join(ns)
        } else {
            audit_dir
        };
        
        if !target_dir.exists() {
            return Ok(Vec::new());
        }
        
        // Read all log files in the directory
        for entry in fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().unwrap_or_default() == "log" {
                // Extract date from filename if needed for filtering
                // For now, we're just reading all log files
                
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                
                for line_result in reader.lines() {
                    let line = line_result?;
                    
                    // Parse the log entry
                    let log_entry: AuditLogEntry = serde_json::from_str(&line)
                        .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
                    
                    // Apply event_type filter if specified
                    if let Some(et) = event_type {
                        if log_entry.action != et {
                            continue;
                        }
                    }
                    
                    // Convert AuditLogEntry to StorageEvent
                    let event = StorageEvent {
                        event_type: log_entry.action,
                        user_id: log_entry.user_id,
                        namespace: log_entry.namespace,
                        key: log_entry.key.unwrap_or_default(),
                        timestamp: log_entry.timestamp,
                        details: log_entry.details,
                    };
                    
                    events.push(event);
                    
                    // Check if we've reached the limit
                    if events.len() >= limit {
                        break;
                    }
                }
                
                // Check if we've reached the limit after processing a file
                if events.len() >= limit {
                    break;
                }
            }
        }
        
        Ok(events)
    }

    fn get_versioned(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<(Vec<u8>, VersionInfo)> {
        // Check the user has permission to read from this namespace
        self.check_permission(auth, "read", namespace)?;
        
        // Check if namespace exists
        if !self.namespace_exists(namespace) {
            return Err(StorageError::NotFound {
                key: format!("{}:{}", namespace, key),
            });
        }
        
        // Read the metadata to get the current version
        let metadata = self.read_key_metadata(namespace, key)?;
        
        // Get the latest version number
        let latest_version = metadata.versions.last().ok_or_else(|| StorageError::NotFound {
            key: format!("{}:{}", namespace, key),
        })?.version;
        
        // Read the version data
        let data = self.read_version_data(namespace, key, latest_version)?;
        
        // Get the version info
        let version_info = metadata.versions.last().ok_or_else(|| StorageError::NotFound {
            key: format!("{}:{}", namespace, key),
        })?.clone();
        
        // Record the audit event
        self.record_audit_log(
            auth,
            "read",
            namespace,
            Some(key),
            &format!("Read versioned data (v{})", latest_version),
        )?;
        
        Ok((data, version_info))
    }

    // Implement the public trait method by delegating to our internal method
    fn check_permission(&self, auth: &AuthContext, action: &str, namespace: &str) -> StorageResult<()> {
        self.check_permission_internal(auth, action, namespace)
    }
}
