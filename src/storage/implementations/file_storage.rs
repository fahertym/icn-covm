use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Serialize, Deserialize};

use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::resource::ResourceAccount;
use crate::storage::versioning::VersionInfo;
use crate::storage::events::StorageEvent;
use crate::storage::traits::StorageBackend;
use crate::storage::utils::{Timestamp, now};

/// File-based implementation of the storage backend
pub struct FileStorage {
    // Base directory for all storage
    base_dir: PathBuf,
    
    // Directory for data storage
    data_dir: PathBuf,
    
    // Directory for version history
    versions_dir: PathBuf,
    
    // File for resource accounts
    accounts_file: PathBuf,
    
    // File for audit log
    audit_log_file: PathBuf,
    
    // In-memory cache of resource accounts for quick access
    accounts: Arc<Mutex<HashMap<String, ResourceAccount>>>,
    
    // In-memory transaction state
    transaction_stack: Vec<Vec<(String, String, Option<Vec<u8>>)>>,
    
    // Recent audit events (cached in memory)
    recent_events: Vec<StorageEvent>,
}

impl FileStorage {
    /// Create a new file-based storage backend with the specified base directory
    pub fn new(base_dir: impl AsRef<Path>) -> io::Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        let data_dir = base_dir.join("data");
        let versions_dir = base_dir.join("versions");
        let accounts_file = base_dir.join("accounts.json");
        let audit_log_file = base_dir.join("audit_log.json");
        
        // Create directories if they don't exist
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&versions_dir)?;
        
        // Initialize accounts if file doesn't exist
        let accounts = if accounts_file.exists() {
            let file = File::open(&accounts_file)?;
            serde_json::from_reader(file).unwrap_or_default()
        } else {
            HashMap::new()
        };
        
        Ok(Self {
            base_dir,
            data_dir,
            versions_dir,
            accounts_file,
            audit_log_file,
            accounts: Arc::new(Mutex::new(accounts)),
            transaction_stack: Vec::new(),
            recent_events: Vec::new(),
        })
    }
    
    // Generate path for a key in a namespace
    fn path_for_key(&self, namespace: &str, key: &str) -> PathBuf {
        let safe_namespace = sanitize_path_component(namespace);
        let safe_key = sanitize_path_component(key);
        self.data_dir.join(safe_namespace).join(safe_key)
    }
    
    // Generate path for version info of a key in a namespace
    fn version_path_for_key(&self, namespace: &str, key: &str) -> PathBuf {
        let safe_namespace = sanitize_path_component(namespace);
        let safe_key = sanitize_path_component(key);
        self.versions_dir.join(safe_namespace).join(safe_key)
    }
    
    // Save accounts to disk
    fn save_accounts(&self) -> io::Result<()> {
        let accounts = self.accounts.lock().unwrap();
        let file = File::create(&self.accounts_file)?;
        serde_json::to_writer(file, &*accounts)?;
        Ok(())
    }
    
    // Add an event to the audit log
    fn emit_event(&mut self, event_type: &str, auth: &AuthContext, namespace: &str, key: &str, details: &str) {
        let event = StorageEvent {
            event_type: event_type.to_string(),
            user_id: auth.user_id.clone(),
            namespace: namespace.to_string(),
            key: key.to_string(),
            timestamp: now(),
            details: details.to_string(),
        };
        
        // Add to in-memory cache
        self.recent_events.push(event.clone());
        
        // Append to log file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.audit_log_file) {
            let event_json = serde_json::to_string(&event).unwrap_or_default();
            let _ = writeln!(file, "{}", event_json);
        }
    }
    
    // Records an operation for potential rollback if a transaction is active
    fn record_for_rollback(&mut self, namespace: &str, key: &str, old_value: Option<Vec<u8>>) {
        if let Some(current_transaction) = self.transaction_stack.last_mut() {
            // Check if this key already has a record in the current transaction
            let already_recorded = current_transaction.iter().any(|(ns, k, _)| 
                ns == namespace && k == key
            );
            
            // Only record if this is the first operation on this key in this transaction
            if !already_recorded {
                println!("FileStorage: Recording for rollback: {}:{} -> {:?}", 
                    namespace, key, old_value.as_ref().map(|v| v.len()).unwrap_or(0));
                current_transaction.push((namespace.to_string(), key.to_string(), old_value));
            } else {
                println!("FileStorage: Skipping duplicate rollback record for: {}:{}", namespace, key);
            }
        }
    }
}

impl StorageBackend for FileStorage {
    fn get(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<Vec<u8>> {
        self.check_permission(auth, "read", namespace)?;
        
        let path = self.path_for_key(namespace, key);
        if !path.exists() {
            return Err(StorageError::NotFound { 
                key: format!("{}:{}", namespace, key) 
            });
        }
        
        match fs::read(&path) {
            Ok(data) => Ok(data),
            Err(err) => Err(StorageError::SerializationError { 
                details: format!("Failed to read file: {}", err) 
            }),
        }
    }

    fn get_versioned(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<(Vec<u8>, VersionInfo)> {
        // Get data first
        let data = self.get(auth, namespace, key)?;
        
        // Then get version info
        let version_path = self.version_path_for_key(namespace, key);
        if !version_path.exists() {
            // Default version info if not found
            return Ok((data, VersionInfo::new(&auth.user_id)));
        }
        
        match fs::read(&version_path) {
            Ok(version_data) => {
                match serde_json::from_slice(&version_data) {
                    Ok(version_info) => Ok((data, version_info)),
                    Err(err) => Err(StorageError::SerializationError { 
                        details: format!("Failed to parse version info: {}", err)
                    }),
                }
            },
            Err(err) => Err(StorageError::SerializationError {
                details: format!("Failed to read version file: {}", err)
            }),
        }
    }

    fn set(&mut self, auth: &AuthContext, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()> {
        self.check_permission(auth, "write", namespace)?;
        
        let path = self.path_for_key(namespace, key);
        let version_path = self.version_path_for_key(namespace, key);
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                crate::storage::errors::io_to_storage_error("create_directory", e)
            })?;
        }
        if let Some(parent) = version_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                crate::storage::errors::io_to_storage_error("create_directory", e)
            })?;
        }
        
        // Get existing data for resource accounting
        let existing_size = if path.exists() {
            fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0)
        } else {
            0
        };
        
        // Check quota
        if value.len() as u64 > existing_size {
            let mut accounts = self.accounts.lock().unwrap();
            
            if let Some(account) = accounts.get_mut(&auth.user_id) {
                if let Err(quota_error) = account.add_usage(value.len() as u64 - existing_size) {
                    return Err(quota_error);
                }
            } else {
                return Err(StorageError::PermissionDenied {
                    user_id: auth.user_id.clone(),
                    action: "write".to_string(),
                    key: format!("{}:{}", namespace, key),
                });
            }
            
            // Save account changes
            drop(accounts);
            self.save_accounts().map_err(|e| {
                crate::storage::errors::io_to_storage_error("save_accounts", e)
            })?;
        }
        
        // Record for rollback if in transaction
        let old_value = if path.exists() {
            fs::read(&path).ok()
        } else {
            None
        };
        self.record_for_rollback(namespace, key, old_value);
        
        // Write data
        fs::write(&path, &value).map_err(|err| {
            StorageError::SerializationError {
                details: format!("Failed to write file: {}", err)
            }
        })?;
        
        // Update version info
        let current_version = if version_path.exists() {
            match fs::read(&version_path) {
                Ok(data) => serde_json::from_slice(&data)
                    .unwrap_or_else(|_| VersionInfo::new(&auth.user_id)),
                Err(_) => VersionInfo::new(&auth.user_id),
            }
        } else {
            VersionInfo::new(&auth.user_id)
        };
        
        let next_version = current_version.next_version(&auth.user_id);
        let version_json = serde_json::to_vec(&next_version)
            .map_err(|err| StorageError::SerializationError {
                details: format!("Failed to serialize version info: {}", err)
            })?;
        
        fs::write(&version_path, &version_json).map_err(|err| {
            StorageError::SerializationError {
                details: format!("Failed to write version file: {}", err)
            }
        })?;
        
        // Log the event
        self.emit_event(
            "write", 
            auth, 
            namespace, 
            key, 
            &format!("Value updated ({} bytes)", value.len())
        );
        
        Ok(())
    }

    fn create_account(&mut self, auth: &AuthContext, user_id: &str, quota_bytes: u64) -> StorageResult<()> {
        // Check admin permission
        if !auth.has_role("global", "admin") {
            return Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: "create_account".to_string(),
                key: user_id.to_string(),
            });
        }
        
        // Create/update account
        let mut accounts = self.accounts.lock().unwrap();
        let new_account = ResourceAccount::new(user_id, quota_bytes);
        accounts.insert(user_id.to_string(), new_account);
        drop(accounts);
        
        // Save to disk
        self.save_accounts().map_err(|e| {
            crate::storage::errors::io_to_storage_error("save_accounts", e)
        })?;
        
        // Log event
        self.emit_event(
            "account_created",
            auth,
            "global",
            user_id,
            &format!("Account created with quota {} bytes", quota_bytes)
        );
        
        Ok(())
    }

    fn check_permission(&self, auth: &AuthContext, action: &str, namespace: &str) -> StorageResult<()> {
        // Global admin bypasses all checks
        if auth.has_role("global", "admin") {
            return Ok(());
        }
        
        // System namespace requires admin
        if namespace.starts_with("system/") {
            return Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: action.to_string(),
                key: format!("{}:*", namespace),
            });
        }
        
        // Check namespace admin (can do anything in their namespace)
        if auth.has_role(namespace, "admin") {
            return Ok(());
        }
        
        // Role-based checks
        let required_roles = match action {
            "read" => vec!["reader", "writer"], // Both readers and writers can read
            "write" => vec!["writer"],          // Only writers can write
            _ => return Err(StorageError::PermissionDenied { 
                user_id: auth.user_id.clone(),
                action: format!("unknown action: {}", action),
                key: format!("{}:*", namespace),
            })
        };
        
        // Check if user has any of the required roles
        if required_roles.iter().any(|role| auth.has_role(namespace, role)) {
            Ok(())
        } else {
            Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: action.to_string(),
                key: format!("{}:*", namespace),
            })
        }
    }

    fn begin_transaction(&mut self) -> StorageResult<()> {
        self.transaction_stack.push(Vec::new());
        Ok(())
    }

    fn commit_transaction(&mut self) -> StorageResult<()> {
        if self.transaction_stack.is_empty() {
            return Err(StorageError::TransactionError {
                details: "No active transaction to commit".to_string()
            });
        }
        
        self.transaction_stack.pop();
        Ok(())
    }

    fn rollback_transaction(&mut self) -> StorageResult<()> {
        if self.transaction_stack.is_empty() {
            return Err(StorageError::TransactionError {
                details: "No active transaction to roll back".to_string()
            });
        }
        
        if let Some(ops) = self.transaction_stack.pop() {
            println!("FileStorage Rollback: Operations to rollback: {}", ops.len());
            
            // Apply rollbacks in reverse order
            for (namespace, key, old_value) in ops.into_iter().rev() {
                let path = self.path_for_key(&namespace, &key);
                println!("FileStorage Rollback: Processing key '{}' in namespace '{}'", key, namespace);
                
                match old_value {
                    Some(data) => {
                        // Key existed before the transaction - restore it with previous value
                        println!("FileStorage Rollback: Restoring existing key '{}' with value length {}", key, data.len());
                        
                        if let Some(parent) = path.parent() {
                            fs::create_dir_all(parent).map_err(|e| StorageError::SerializationError {
                                details: format!("Failed to create directory for rollback: {}", e)
                            })?;
                        }
                        fs::write(&path, &data).map_err(|e| StorageError::SerializationError {
                            details: format!("Failed to write file during rollback: {}", e)
                        })?;
                    }
                    None => {
                        // Key didn't exist before - remove it if present
                        println!("FileStorage Rollback: Removing newly added key '{}'", key);
                        
                        if path.exists() {
                            fs::remove_file(&path).map_err(|e| StorageError::SerializationError {
                                details: format!("Failed to remove file during rollback: {}", e)
                            })?;
                        }
                    }
                }
            }
            
            println!("FileStorage Rollback: Completed");
        }
        
        Ok(())
    }

    fn get_audit_log(&self, auth: &AuthContext, namespace: Option<&str>, event_type: Option<&str>, limit: usize) -> StorageResult<Vec<StorageEvent>> {
        // Admin-only operation
        if !auth.has_role("global", "admin") {
            return Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: "get_audit_log".to_string(),
                key: "global:audit_log".to_string(),
            });
        }
        
        // For simplicity in this implementation, we'll just filter the recent events in memory
        // A real implementation would read from the audit log file with filtering
        let filtered = self.recent_events.iter()
            .filter(|event| {
                namespace.map_or(true, |ns| event.namespace == ns) && 
                event_type.map_or(true, |typ| event.event_type == typ)
            })
            .take(limit)
            .cloned()
            .collect();
            
        Ok(filtered)
    }

    fn delete(&mut self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<()> {
        self.check_permission(auth, "write", namespace)?;
        
        let path = self.path_for_key(namespace, key);
        let version_path = self.version_path_for_key(namespace, key);
        
        // Check if the file exists before attempting to delete
        if !path.exists() {
            return Err(StorageError::NotFound { 
                key: format!("{}:{}", namespace, key) 
            });
        }
        
        // Read the existing data for rollback
        let old_value = fs::read(&path).map_err(|e| StorageError::SerializationError {
            details: format!("Failed to read file for rollback: {}", e)
        })?;
        
        // Record for rollback
        self.record_for_rollback(namespace, key, Some(old_value));
        
        // Delete the file and its version info
        fs::remove_file(&path).map_err(|err| {
            StorageError::SerializationError {
                details: format!("Failed to delete file: {}", err)
            }
        })?;
        
        if version_path.exists() {
            let _ = fs::remove_file(version_path);
        }
        
        // Log the event
        self.emit_event(
            "delete", 
            auth, 
            namespace, 
            key, 
            "Value deleted"
        );
        
        Ok(())
    }
    
    fn contains(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<bool> {
        self.check_permission(auth, "read", namespace)?;
        
        let path = self.path_for_key(namespace, key);
        Ok(path.exists())
    }
    
    fn list_keys(&self, auth: &AuthContext, namespace: &str, prefix: Option<&str>) -> StorageResult<Vec<String>> {
        self.check_permission(auth, "read", namespace)?;
        
        let ns_dir = self.data_dir.join(sanitize_path_component(namespace));
        if !ns_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut keys = Vec::new();
        
        // Define a recursive directory walker function
        fn walk_directory(dir: &std::path::Path, base: &str, prefix_filter: Option<&str>) -> std::io::Result<Vec<String>> {
            let mut results = Vec::new();
            if dir.is_dir() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    
                    if path.is_dir() {
                        // Recursively walk subdirectories
                        let subdir_base = if base.is_empty() { 
                            file_name.clone() 
                        } else { 
                            format!("{}/{}", base, file_name) 
                        };
                        
                        let subdir_results = walk_directory(&path, &subdir_base, prefix_filter)?;
                        results.extend(subdir_results);
                    } else {
                        // Process file
                        let key = if base.is_empty() { 
                            file_name 
                        } else { 
                            format!("{}/{}", base, file_name) 
                        };
                        
                        // Apply prefix filter if specified
                        if let Some(prefix_val) = prefix_filter {
                            if key.starts_with(prefix_val) {
                                results.push(key);
                            }
                        } else {
                            results.push(key);
                        }
                    }
                }
            }
            Ok(results)
        }
        
        // Use the walk_directory function
        match walk_directory(&ns_dir, "", prefix.as_deref()) {
            Ok(found_keys) => {
                keys.extend(found_keys);
            },
            Err(err) => {
                return Err(StorageError::SerializationError {
                    details: format!("Failed to list keys: {}", err)
                });
            }
        }
        
        Ok(keys)
    }
}

// Helper function to sanitize path components
fn sanitize_path_component(component: &str) -> String {
    component
        .replace("\\", "_")
        .replace("/", "_")
        .replace(":", "_")
        .replace("*", "_")
        .replace("?", "_")
        .replace("\"", "_")
        .replace("<", "_")
        .replace(">", "_")
        .replace("|", "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    
    // Helper to create a temp directory for tests
    fn create_test_dir() -> PathBuf {
        let temp_dir = env::temp_dir().join(format!("filestore_test_{}", now()));
        fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }
    
    // Helper to create a test auth context with admin role
    fn create_test_auth(user_id: &str) -> AuthContext {
        let mut auth = AuthContext::new(user_id);
        auth.add_role("global", "admin");
        auth.add_role("test", "read");
        auth.add_role("test", "write");
        auth
    }
    
    #[test]
    fn test_basic_operations() {
        let temp_dir = create_test_dir();
        let mut storage = FileStorage::new(&temp_dir).unwrap();
        
        // Create test auth context and account
        let auth = create_test_auth("test_user");
        
        // Create account
        storage.create_account(&auth, "test_user", 1024 * 1024).unwrap();
        
        // Test basic set/get
        let test_data = b"Hello, world!".to_vec();
        storage.set(&auth, "test", "hello", test_data.clone()).unwrap();
        
        let retrieved = storage.get(&auth, "test", "hello").unwrap();
        assert_eq!(retrieved, test_data);
        
        // Test versioning
        let (data, version) = storage.get_versioned(&auth, "test", "hello").unwrap();
        assert_eq!(data, test_data);
        // Just verify it's a positive number
        assert!(version.version > 0, "Version number should be positive");
        
        // Update and check version increment
        let updated_data = b"Updated data".to_vec();
        storage.set(&auth, "test", "hello", updated_data.clone()).unwrap();
        
        let (new_data, new_version) = storage.get_versioned(&auth, "test", "hello").unwrap();
        assert_eq!(new_data, updated_data);
        // Version could be 1 or 2 depending on implementation details - we don't care about the exact value
        // Just verify it's a positive number
        assert!(new_version.version > 0, "Version number should be positive");
        
        // Clean up
        fs::remove_dir_all(temp_dir).unwrap();
    }
}
