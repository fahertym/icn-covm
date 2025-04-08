use std::io::{Write, BufRead, BufReader};
use std::fs::{self, File, create_dir_all, OpenOptions};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use crate::storage::auth::AuthContext;
use crate::storage::traits::StorageBackend;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::versioning::{VersionInfo, VersionDiff, DiffChange};
use crate::storage::events::StorageEvent;
use crate::storage::namespaces::NamespaceMetadata;
use crate::storage::utils::{Timestamp, now};

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
    account_cache: HashMap<String, FileResourceAccount>,
}

/// Represents a user's resource account for storage quota management
#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileResourceAccount {
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
                let account: FileResourceAccount = serde_json::from_str(&account_str)
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
        
        // Open the file with read-only access
        let file = File::open(&path)?;
        
        // Acquire a shared lock for reading
        file.lock_shared()?;
        
        // Read the file content
        let metadata_str = fs::read_to_string(path)?;
        
        // Parse the JSON
        let metadata: KeyMetadata = serde_json::from_str(&metadata_str)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        // The lock will be automatically released when the file is closed
        
        Ok(metadata)
    }
    
    /// Writes key metadata to disk
    fn write_key_metadata(&self, namespace: &str, key: &str, metadata: &KeyMetadata) -> StorageResult<()> {
        let path = self.metadata_path(namespace, key);
        
        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        
        // Serialize metadata to JSON
        let metadata_str = serde_json::to_string(metadata)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        // Open the file with write access, creating it if it doesn't exist
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        
        // Acquire an exclusive lock for writing
        file.lock_exclusive()?;
        
        // Write the metadata
        fs::write(path, metadata_str)?;
        
        // The lock will be automatically released when the file is closed
        
        Ok(())
    }
    
    /// Writes data to a specific version file
    fn write_version_data(&self, namespace: &str, key: &str, version: u64, data: &[u8]) -> StorageResult<()> {
        let path = self.version_path(namespace, key, version);
        
        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        
        // Open the file with write access, creating it if it doesn't exist
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        
        // Acquire an exclusive lock for writing
        file.lock_exclusive()?;
        
        // Write the data
        fs::write(path, data)?;
        
        // The lock will be automatically released when the file is closed
        
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
        
        // Open the file with read access
        let file = File::open(&path)?;
        
        // Acquire a shared lock for reading
        file.lock_shared()?;
        
        // Read the data
        let data = fs::read(path)?;
        
        // The lock will be automatically released when the file is closed
        
        Ok(data)
    }
    
    /// Writes a namespace metadata file
    fn write_namespace_metadata(&self, metadata: &NamespaceMetadata) -> StorageResult<()> {
        let path = self.namespace_metadata_path(&metadata.path);
        
        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        
        // Serialize metadata to JSON
        let metadata_str = serde_json::to_string(metadata)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        // Open the file with write access, creating it if it doesn't exist
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        
        // Acquire an exclusive lock for writing
        file.lock_exclusive()?;
        
        // Write the metadata
        fs::write(path, metadata_str)?;
        
        // The lock will be automatically released when the file is closed
        
        Ok(())
    }
    
    /// Records an audit log entry
    fn record_audit_log(&self, auth: &AuthContext, action: &str, namespace: &str, key: Option<&str>, message: &str) -> StorageResult<()> {
        let now = now();
        let date = DateTime::<Utc>::from_timestamp(now as i64, 0)
            .ok_or_else(|| StorageError::TransactionError { details: "Invalid timestamp".to_string() })?
            .format("%Y%m%d").to_string();
        
        let log_path = self.root_path.join("audit_logs").join(format!("log_{}.jsonl", date));
        
        let log_entry = AuditLogEntry {
            timestamp: now,
            user_id: auth.user_id.clone(),
            action: action.to_string(),
            namespace: namespace.to_string(),
            key: key.map(String::from),
            details: message.to_string(),
        };
        
        let log_str = serde_json::to_string(&log_entry)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        
        // Append to log file with proper locking
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        
        // Acquire an exclusive lock for appending
        file.lock_exclusive()?;
        
        // Write the log entry
        writeln!(file, "{}", log_str)?;
        
        // The lock will be automatically released when the file is closed
        
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
    fn get(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<Vec<u8>> {
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
        self.record_audit_log(auth.as_ref().unwrap_or_else(|| panic!("Auth required for audit log")), "read", namespace, Some(key), &format!("Read version {}", latest_version.version))?;
        
        Ok(data)
    }
    
    fn set(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()> {
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
            let user_id = auth.map(|a| a.user_id.clone()).unwrap_or_else(|| "system".to_string());
            if let Some(account) = self.account_cache.get_mut(&user_id) {
                if account.used_bytes + additional_bytes > account.quota_bytes {
                    return Err(StorageError::QuotaExceeded {
                        account_id: user_id.clone(),
                        requested: additional_bytes,
                        available: account.quota_bytes.saturating_sub(account.used_bytes),
                    });
                }
                
                // Update the account usage
                account.used_bytes += additional_bytes;
                
                // Save the updated account
                let account_path = self.root_path.join("accounts").join(format!("{}.json", user_id));
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
        
        // Get the user ID for the version info
        let user_id = auth.map(|a| a.user_id.clone()).unwrap_or_else(|| "system".to_string());
        
        // Get the new version number
        let version_info = if key_metadata_exists {
            let mut metadata = self.read_key_metadata(namespace, key)?;
            let latest_version = metadata.versions.last()
                .map(|v| v.version)
                .unwrap_or(0);
            
            let new_version = latest_version + 1;
            let version_info = VersionInfo {
                version: new_version,
                created_by: user_id.clone(),
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
                created_by: user_id.clone(),
                timestamp: now(),
                prev_version: None,
            };
            
            let metadata = KeyMetadata {
                key: key.to_string(),
                created_by: user_id.clone(),
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
            auth.as_ref().unwrap_or_else(|| panic!("Auth required for audit log")),
            "write",
            namespace,
            Some(key),
            &format!("Set v{} ({} bytes)", version_info.version, value_size),
        )?;
        
        Ok(())
    }
    
    fn delete(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<()> {
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
        self.record_audit_log(
            auth.as_ref().unwrap_or_else(|| panic!("Auth required for audit log")),
            "delete",
            namespace,
            Some(key),
            &format!("Deleted version {}", latest_version.version),
        )?;
        
        Ok(())
    }
    
    fn list_keys(&self, auth: Option<&AuthContext>, namespace: &str, prefix: Option<&str>) -> StorageResult<Vec<String>> {
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
        self.record_audit_log(
            auth.as_ref().unwrap_or_else(|| panic!("Auth required for audit log")),
            "list_keys",
            namespace,
            None,
            &format!("Listed keys with prefix {:?}", prefix),
        )?;
        
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
    
    fn get_version(&self, auth: Option<&AuthContext>, namespace: &str, key: &str, version: u64) -> StorageResult<(Vec<u8>, VersionInfo)> {
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
        self.record_audit_log(
            auth.as_ref().unwrap_or_else(|| panic!("Auth required for audit log")),
            "read_version",
            namespace,
            Some(key),
            &format!("Read version {}", version),
        )?;
        
        Ok((data, version_info.clone()))
    }
    
    fn list_versions(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<Vec<VersionInfo>> {
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
        self.record_audit_log(
            auth.as_ref().unwrap_or_else(|| panic!("Auth required for audit log")),
            "list_versions",
            namespace,
            Some(key),
            &format!("Listed versions for key"),
        )?;
        
        Ok(metadata.versions)
    }
    
    fn diff_versions(&self, auth: Option<&AuthContext>, namespace: &str, key: &str, v1: u64, v2: u64) -> StorageResult<VersionDiff<Vec<u8>>> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;
        
        // Get the data for version 1
        let (data1, info1) = self.get_version(auth, namespace, key, v1)?;
        
        // Get the data for version 2
        let (data2, info2) = self.get_version(auth, namespace, key, v2)?;
        
        // Record the audit event
        self.record_audit_log(
            auth.as_ref().unwrap_or_else(|| panic!("Auth required for audit log")),
            "diff_versions",
            namespace,
            Some(key),
            &format!("Diffed versions {} and {}", v1, v2),
        )?;
        
        // Implementation of diff can vary, but here's a basic one
        // Just indicate if the entire value changed
        let mut changes = Vec::new();
        
        if data1 != data2 {
            changes.push(DiffChange::ValueChanged {
                path: "data".to_string(),
                old_value: data1.clone(),
                new_value: data2.clone(),
            });
        }
        
        // Create a user ID for the diff creator
        let user_id = auth.map(|a| a.user_id.clone()).unwrap_or_else(|| "system".to_string());
        
        // Return the diff
        Ok(VersionDiff {
            old_version: v1,
            new_version: v2,
            created_by: user_id,
            timestamp: now(),
            changes,
        })
    }
    
    fn create_namespace(&mut self, auth: Option<&AuthContext>, namespace: &str, quota_bytes: u64, parent_namespace: Option<&str>) -> StorageResult<()> {
        // Check if user has admin permission on global or parent namespace
        let can_create = auth.map_or(false, |a| a.has_role("global", "admin") || 
            parent_namespace.map_or(false, |p| a.has_role(p, "admin")));
            
        if !can_create {
            return Err(StorageError::PermissionDenied {
                user_id: auth.map_or("anonymous".to_string(), |a| a.user_id.clone()),
                action: "create_namespace".to_string(),
                key: namespace.to_string(),
            });
        }
        
        // Check if parent exists when specified
        if let Some(parent_ns) = parent_namespace {
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
            owner: auth.map_or("SYSTEM".to_string(), |a| a.user_id.clone()),
            quota_bytes,
            used_bytes: 0,
            parent: parent_namespace.map(String::from),
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
        if let Some(auth_ref) = auth {
            self.record_audit_log(auth_ref, "create_namespace", namespace, None,
                &format!("Created namespace with quota {} bytes", quota_bytes))?;
        }
        
        Ok(())
    }
    
    fn list_namespaces(&self, auth: Option<&AuthContext>, parent_namespace: &str) -> StorageResult<Vec<crate::storage::namespaces::NamespaceMetadata>> {
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
            if auth.map_or(false, |a| a.has_role(path, "reader") || 
               a.has_role(path, "writer") || 
               a.has_role(path, "admin") ||
               a.has_role("global", "admin")) {
                namespaces.push(metadata.clone());
            }
        }
        
        // Record audit log
        if let Some(auth_ref) = auth {
            self.record_audit_log(auth_ref, "list_namespaces", parent_namespace, None,
                &format!("Listed namespaces (found {})", namespaces.len()))?;
        }
        
        Ok(namespaces)
    }
    
    fn get_usage(&self, auth: Option<&AuthContext>, namespace: &str) -> StorageResult<u64> {
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
    
    fn create_account(&mut self, auth: Option<&AuthContext>, user_id: &str, quota_bytes: u64) -> StorageResult<()> {
        // Check admin permissions
        self.check_permission(auth, "admin", "global")?;
        
        // Create accounts directory if it doesn't exist
        let accounts_dir = self.root_path.join("accounts");
        create_dir_all(&accounts_dir)?;
        
        // Check if account already exists
        let account_path = accounts_dir.join(format!("{}.json", user_id));
        if account_path.exists() {
            return Err(StorageError::TransactionError {
                details: format!("Account already exists for user {}", user_id),
            });
        }
        
        // Create resource account
        let account = FileResourceAccount {
            user_id: user_id.to_string(),
            quota_bytes,
            used_bytes: 0,
            created_at: now(),
            last_updated: now(),
        };
        
        // Store it in cache
        self.account_cache.insert(user_id.to_string(), account.clone());
        
        // Serialize to JSON and write to file
        let account_json = match serde_json::to_string_pretty(&account) {
            Ok(json) => json,
            Err(e) => return Err(StorageError::SerializationError { 
                details: format!("Failed to serialize account: {}", e) 
            }),
        };
        
        fs::write(account_path, account_json)?;
        
        // Record audit log
        self.record_audit_log(
            auth.as_ref().unwrap_or_else(|| panic!("Auth required for audit log")),
            "account_created",
            "global",
            Some(user_id),
            &format!("Created account with {} byte quota", quota_bytes),
        )?;
        
        Ok(())
    }
    
    fn get_audit_log(&self, auth: Option<&AuthContext>, namespace: Option<&str>, event_type: Option<&str>, limit: usize) -> StorageResult<Vec<StorageEvent>> {
        // Permission Check: Only global admin or namespace admin (for that namespace)
        let effective_ns = namespace.unwrap_or("global");
        if !auth.unwrap().has_role("global", "admin") && !auth.unwrap().has_role(effective_ns, "admin") {
            return Err(StorageError::PermissionDenied {
                user_id: auth.unwrap().user_id.clone(),
                action: "view_audit_log".to_string(),
                key: effective_ns.to_string(),
            });
        }
        
        // Get log file
        let log_path = self.root_path.join("audit_logs").join("audit.log");
        if !log_path.exists() {
            return Ok(Vec::new());
        }
        
        // Read log file
        let file = File::open(log_path)?;
        let reader = BufReader::new(file);
        
        // Parse events
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if let Ok(event) = serde_json::from_str::<StorageEvent>(&line) {
                // Filter by namespace
                if let Some(ns) = namespace {
                    if event.namespace != ns {
                        continue;
                    }
                }
                
                // Filter by event type
                if let Some(et) = event_type {
                    if event.event_type != et {
                        continue;
                    }
                }
                
                events.push(event);
                
                // Limit results
                if events.len() >= limit {
                    break;
                }
            }
        }
        
        Ok(events)
    }

    fn get_versioned(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<(Vec<u8>, VersionInfo)> {
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
        if let Some(auth_ref) = auth {
            self.record_audit_log(auth_ref, "read", namespace, Some(key),
                &format!("Read versioned data (v{})", latest_version)
            )?;
        }
        
        Ok((data, version_info))
    }

    // Implement the public trait method by delegating to our internal method
    fn check_permission(&self, auth: Option<&AuthContext>, action: &str, namespace: &str) -> StorageResult<()> {
        match auth {
            Some(auth) => self.check_permission_internal(auth, action, namespace),
            None => Err(StorageError::PermissionDenied {
                user_id: "anonymous".to_string(),
                action: action.to_string(),
                key: namespace.to_string(),
            }),
        }
    }

    fn contains(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<bool> {
        // Verify user has read permission for the namespace
        self.check_permission(auth, "read", namespace)?;
        
        // Check if the namespace exists first
        if !self.namespace_exists(namespace) {
            return Ok(false);
        }
        
        // Check if the key metadata file exists
        let metadata_path = self.metadata_path(namespace, key);
        
        Ok(metadata_path.exists())
    }
}
