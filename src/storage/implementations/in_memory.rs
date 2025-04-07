use std::collections::HashMap;
use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::resource::ResourceAccount;
use crate::storage::versioning::VersionInfo;
use crate::storage::events::StorageEvent;
use crate::storage::traits::StorageBackend;
use crate::storage::utils::{Timestamp, now};
use serde::{Serialize, de::DeserializeOwned};

/// An in-memory implementation of the `StorageBackend` trait.
/// Suitable for testing and demos.
#[derive(Default, Debug)]
pub struct InMemoryStorage {
    // Namespace -> Key -> Value
    data: HashMap<String, HashMap<String, Vec<u8>>>,
    // Namespace -> Key -> VersionInfo
    versions: HashMap<String, HashMap<String, VersionInfo>>,
    // User ID -> ResourceAccount
    accounts: HashMap<String, ResourceAccount>,
    // Audit log
    audit_log: Vec<StorageEvent>,
    // Transaction support: Stack of operations to rollback
    // Each operation is (namespace, key, Option<old_value>)
    // None means the key didn't exist before the transaction started.
    transaction_stack: Vec<Vec<(String, String, Option<Vec<u8>>)>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }

    // Helper to create a combined key for internal maps
    fn make_internal_key(namespace: &str, key: &str) -> String {
        // Simple concatenation, might need more robust namespacing
        format!("{}:{}", namespace, key)
    }

    // Records an operation for potential rollback if a transaction is active
    fn record_for_rollback(&mut self, namespace: &str, key: &str, old_value: Option<Vec<u8>>) {
        if let Some(current_transaction) = self.transaction_stack.last_mut() {
            // Remove the check for already recorded operations
            // We want to record *all* operations in the exact order they occurred
            // so we can properly rollback in reverse order
            println!("Recording for rollback: {}:{} -> {:?}", namespace, key, 
                old_value.as_ref().map(|v| v.len()).unwrap_or(0));
            current_transaction.push((namespace.to_string(), key.to_string(), old_value));
        }
    }

    // Emit an event to the audit log
    fn emit_event(&mut self, event_type: &str, auth: &AuthContext, namespace: &str, key: &str, details: &str) {
        // TODO: Consider making event emission configurable or optional
        self.audit_log.push(StorageEvent {
            event_type: event_type.to_string(),
            user_id: auth.user_id.clone(),
            namespace: namespace.to_string(),
            key: key.to_string(),
            timestamp: now(),
            details: details.to_string(),
        });
    }

    /// Helper method to set data by serializing a Rust type into JSON.
    /// This is implemented directly on InMemoryStorage, not part of the trait.
    pub fn set_json<T: Serialize>(&mut self, auth: &AuthContext, namespace: &str, key: &str, value: &T) -> StorageResult<()> {
        let serialized = serde_json::to_vec(value).map_err(|e| StorageError::SerializationError {
            details: e.to_string(),
        })?;
        // Call the trait method `set` internally
        self.set(auth, namespace, key, serialized)
    }

    /// Helper method to get data by deserializing JSON into a Rust type.
    /// This is implemented directly on InMemoryStorage, not part of the trait.
    pub fn get_json<T: DeserializeOwned>(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<T> {
        // Call the trait method `get` internally
        let data = self.get(auth, namespace, key)?;
        serde_json::from_slice(&data).map_err(|e| StorageError::SerializationError {
            details: e.to_string(),
        })
    }
}

impl StorageBackend for InMemoryStorage {
    fn get(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<Vec<u8>> {
        self.check_permission(auth, "read", namespace)?;

        self.data
            .get(namespace)
            .and_then(|ns_data| ns_data.get(key))
            .cloned()
            .ok_or_else(|| StorageError::NotFound {
                key: Self::make_internal_key(namespace, key),
            })
    }

    fn get_versioned(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<(Vec<u8>, VersionInfo)> {
        // Reuse the basic get for data and permission check
        let data = self.get(auth, namespace, key)?;

        let version_info = self.versions
            .get(namespace)
            .and_then(|ns_versions| ns_versions.get(key))
            .cloned()
            .ok_or_else(|| StorageError::NotFound { // Should be consistent with get()
                key: Self::make_internal_key(namespace, key),
            })?;
        
        Ok((data, version_info))
    }

    fn set(&mut self, auth: &AuthContext, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()> {
        self.check_permission(auth, "write", namespace)?;

        let value_size = value.len() as u64;
        let internal_key = Self::make_internal_key(namespace, key);

        // Get existing data for rollback and resource accounting
        let existing_value = self.data.get(namespace).and_then(|ns| ns.get(key)).cloned();
        let existing_size = existing_value.as_ref().map(|v| v.len() as u64).unwrap_or(0);

        // Record for potential rollback *before* making changes
        self.record_for_rollback(namespace, key, existing_value);

        // Resource Accounting Check
        if value_size > existing_size {
            let additional_bytes = value_size - existing_size;
            let account = self.accounts.get_mut(&auth.user_id)
                .ok_or_else(|| StorageError::PermissionDenied {
                    user_id: auth.user_id.clone(),
                    action: "write (no account)".to_string(), // Better error?
                    key: internal_key.clone(),
            })?;
            account.add_usage(additional_bytes)?;
        } else if value_size < existing_size {
             let reduced_bytes = existing_size - value_size;
             if let Some(account) = self.accounts.get_mut(&auth.user_id) {
                 account.reduce_usage(reduced_bytes);
             } // else: Ignore if user has no account? Or error?
        }

        // Update Data
        let ns_data = self.data.entry(namespace.to_string()).or_default();
        ns_data.insert(key.to_string(), value);

        // Update Version
        let ns_versions = self.versions.entry(namespace.to_string()).or_default();
        let current_version = ns_versions.get(key);
        let next_version = match current_version {
            Some(v) => v.next_version(&auth.user_id),
            None => VersionInfo::new(&auth.user_id),
        };
        ns_versions.insert(key.to_string(), next_version);

        // Emit Audit Event
        self.emit_event(
            "write",
            auth,
            namespace,
            key,
            &format!("Value updated ({} bytes)", value_size),
        );

        Ok(())
    }

    // set_json and get_json use default implementations from the trait

    fn create_account(&mut self, auth: &AuthContext, user_id: &str, quota_bytes: u64) -> StorageResult<()> {
        // Permission Check: Only global admins can create accounts
        if !auth.has_role("global", "admin") {
            return Err(StorageError::PermissionDenied {
                user_id: auth.user_id.clone(),
                action: "create_account".to_string(),
                key: user_id.to_string(),
            });
        }

        if self.accounts.contains_key(user_id) {
            // Optionally allow updating quota? For now, return error if exists.
            return Err(StorageError::TransactionError { // Better error type?
                details: format!("Account already exists for user {}", user_id)
            });
        }

        let new_account = ResourceAccount::new(user_id, quota_bytes);
        self.accounts.insert(user_id.to_string(), new_account);

        self.emit_event(
            "account_created",
            auth,
            "global", // Account creation is a global event
            user_id,
            &format!("Account created with quota {} bytes", quota_bytes),
        );

        Ok(())
    }

    // Internal permission logic reused by get/set/etc.
    fn check_permission(&self, auth: &AuthContext, action: &str, namespace: &str) -> StorageResult<()> {
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
            "write" => vec!["writer"],         // Only writers can write
            // Add other actions like "delete", "administer"?
            _ => return Err(StorageError::PermissionDenied { // Unknown action
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

    fn begin_transaction(&mut self) -> StorageResult<()> {
        self.transaction_stack.push(Vec::new());
        Ok(())
    }

    fn commit_transaction(&mut self) -> StorageResult<()> {
        if self.transaction_stack.pop().is_none() {
            Err(StorageError::TransactionError {
                details: "No active transaction to commit".to_string(),
            })
        } else {
            // Just discard the rollback log on commit
            Ok(())
        }
    }

    fn rollback_transaction(&mut self) -> StorageResult<()> {
        match self.transaction_stack.pop() {
            Some(ops) => {
                println!("Rollback: Operations to rollback: {}", ops.len());
                
                // Apply rollbacks in reverse order
                for (namespace, key, old_value_opt) in ops.into_iter().rev() {
                    println!("Rollback: Processing key '{}' in namespace '{}'", key, namespace);
                    
                    match old_value_opt {
                        Some(old_value) => {
                            // Key existed before transaction or was modified during transaction - restore its previous value
                            println!("Rollback: Restoring existing key '{}' with value length {}", key, old_value.len());
                            
                            // Ensure namespace exists
                            let ns_data = self.data.entry(namespace.clone()).or_default();
                            // Put the old value back
                            ns_data.insert(key, old_value);
                            
                            // TODO: Rollback version info? This is complex.
                            // TODO: Rollback resource account usage?
                        }
                        None => {
                            // Key didn't exist before the operation - remove it completely
                            println!("Rollback: Removing newly added key '{}'", key);
                            
                            if let Some(ns_data) = self.data.get_mut(&namespace) {
                                ns_data.remove(&key);
                                
                                // If namespace is now empty, remove it too
                                if ns_data.is_empty() {
                                    println!("Rollback: Removing empty namespace '{}'", namespace);
                                    self.data.remove(&namespace);
                                }
                            }
                            // TODO: Rollback version info?
                            // TODO: Rollback resource account usage?
                        }
                    }
                }
                
                // Print the current state after rollback
                let namespaces = self.data.keys().cloned().collect::<Vec<_>>();
                println!("After rollback: Namespaces: {:?}", namespaces);
                for ns in &namespaces {
                    if let Some(ns_data) = self.data.get(ns) {
                        println!("After rollback: Keys in '{}': {:?}", ns, ns_data.keys().collect::<Vec<_>>());
                    }
                }
                
                Ok(())
            }
            None => Err(StorageError::TransactionError {
                details: "No active transaction to rollback".to_string(),
            }),
        }
    }

    fn get_audit_log(&self, auth: &AuthContext, namespace: Option<&str>, event_type: Option<&str>, limit: usize) -> StorageResult<Vec<StorageEvent>> {
         // Permission Check: Only global admin or namespace admin (for that namespace)
         let effective_ns = namespace.unwrap_or("global");
         if !auth.has_role("global", "admin") && !auth.has_role(effective_ns, "admin") {
             return Err(StorageError::PermissionDenied {
                 user_id: auth.user_id.clone(),
                 action: "view_audit_log".to_string(),
                 key: effective_ns.to_string(),
             });
         }
 
         // Filter logic
         let results: Vec<StorageEvent> = self.audit_log.iter()
            .filter(|event| {
                // Namespace filter: If namespace is Some, event must match.
                let ns_match = namespace.map_or(true, |ns| event.namespace == ns);
                // Event type filter: If event_type is Some, event must match.
                let type_match = event_type.map_or(true, |et| event.event_type == et);
                ns_match && type_match
            })
            // Iterate in reverse to get latest events first, then take limit
            .rev()
            .take(limit)
            // Clone events to return owned data
            .cloned()
            // Collect into a Vec
            .collect();

        // Reverse again to restore chronological order if needed, or return as is (latest first).
        // Let's return latest first.
        Ok(results)
    }

    fn delete(&mut self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<()> {
        self.check_permission(auth, "write", namespace)?;

        let internal_key = Self::make_internal_key(namespace, key);
        
        // Get existing value if it exists (for rollback)
        let existing_value = self.data.get(namespace).and_then(|ns| ns.get(key)).cloned();
        
        if existing_value.is_none() {
            // If the key doesn't exist, there's nothing to delete
            return Err(StorageError::NotFound { 
                key: internal_key 
            });
        }
        
        // Record the existing value for rollback before deletion
        self.record_for_rollback(namespace, key, existing_value);
        
        // Delete data
        if let Some(ns_data) = self.data.get_mut(namespace) {
            ns_data.remove(key);
            
            // Clean up empty namespace (though this is optional)
            if ns_data.is_empty() {
                self.data.remove(namespace);
            }
        }
        
        // Emit Audit Event
        self.emit_event(
            "delete",
            auth,
            namespace,
            key,
            "Value deleted",
        );
        
        Ok(())
    }
    
    fn contains(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<bool> {
        self.check_permission(auth, "read", namespace)?;
        
        // Simply check if the key exists in the data map
        // We don't need special transaction handling here - 
        // by the time this is called after a rollback, the transaction is already processed
        Ok(self.data.get(namespace).map_or(false, |ns| ns.contains_key(key)))
    }
    
    fn list_keys(&self, auth: &AuthContext, namespace: &str, prefix: Option<&str>) -> StorageResult<Vec<String>> {
        self.check_permission(auth, "read", namespace)?;
        
        // Get all keys in the namespace
        let keys = self.data.get(namespace)
            .map(|ns| {
                ns.keys()
                    .filter(|k| {
                        // Apply prefix filter if provided
                        if let Some(prefix) = prefix {
                            k.starts_with(prefix)
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
            
        Ok(keys)
    }
}

// Add unit tests for InMemoryStorage here
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::auth::AuthContext;

    #[test]
    fn test_set_get() {
        let mut storage = InMemoryStorage::new();
        let mut auth = AuthContext::new("test_user");
        auth.add_role("test_ns", "writer");
        auth.add_role("test_ns", "reader");

        // Need to create account first
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        storage.create_account(&admin_auth, "test_user", 1000).unwrap();

        let data = vec![1, 2, 3];
        storage.set(&auth, "test_ns", "test_key", data.clone()).unwrap();
        let retrieved = storage.get(&auth, "test_ns", "test_key").unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_permission_denied() {
        let mut storage = InMemoryStorage::new();
        let mut reader_auth = AuthContext::new("reader_user");
        reader_auth.add_role("test_ns", "reader");
        let writer_auth = AuthContext::new("writer_user");

        // Reader tries to write
        let data = vec![4, 5, 6];
        let result = storage.set(&reader_auth, "test_ns", "key1", data.clone());
        assert!(matches!(result, Err(StorageError::PermissionDenied { .. })));

        // Unpermissioned user tries to read (assuming they can't by default)
        // First set data using an admin/writer
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        storage.create_account(&admin_auth, "admin", 100).unwrap(); // Need account for admin too
        storage.set(&admin_auth, "test_ns", "key2", vec![7]).unwrap();

        let result_read = storage.get(&writer_auth, "test_ns", "key2"); // writer_auth has no roles
        assert!(matches!(result_read, Err(StorageError::PermissionDenied { .. })));
    }

     #[test]
    fn test_versioning() {
        let mut storage = InMemoryStorage::new();
        let mut auth = AuthContext::new("v_user");
        auth.add_role("version_ns", "writer");

        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        storage.create_account(&admin_auth, "v_user", 1000).unwrap();

        storage.set(&auth, "version_ns", "v_key", vec![1]).unwrap();
        let (_, v1) = storage.get_versioned(&auth, "version_ns", "v_key").unwrap();
        assert_eq!(v1.version, 1);
        assert_eq!(v1.created_by, "v_user");
        assert!(v1.prev_version.is_none());

        storage.set(&auth, "version_ns", "v_key", vec![2]).unwrap();
        let (_, v2) = storage.get_versioned(&auth, "version_ns", "v_key").unwrap();
        assert_eq!(v2.version, 2);
        assert!(v2.prev_version.is_some());
        assert_eq!(v2.prev_version.unwrap().version, 1);
    }

    #[test]
    fn test_quota() {
        let mut storage = InMemoryStorage::new();
        let mut auth = AuthContext::new("q_user");
        auth.add_role("quota_ns", "writer");

        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        storage.create_account(&admin_auth, "q_user", 50).unwrap(); // 50 byte quota

        // Set 30 bytes - should work
        storage.set(&auth, "quota_ns", "key1", vec![0; 30]).unwrap();
        let account = storage.accounts.get("q_user").unwrap();
        assert_eq!(account.storage_used_bytes, 30);

        // Try to set another 30 bytes (total 60) - should fail
        let result = storage.set(&auth, "quota_ns", "key2", vec![0; 30]);
        assert!(matches!(result, Err(StorageError::QuotaExceeded { .. })));
        let account = storage.accounts.get("q_user").unwrap();
        assert_eq!(account.storage_used_bytes, 30); // Usage shouldn't change

        // Overwrite key1 with 10 bytes (reduces usage)
        storage.set(&auth, "quota_ns", "key1", vec![0; 10]).unwrap();
        let account = storage.accounts.get("q_user").unwrap();
        assert_eq!(account.storage_used_bytes, 10);

        // Now set key2 with 30 bytes (total 40) - should work
        storage.set(&auth, "quota_ns", "key2", vec![0; 30]).unwrap();
        let account = storage.accounts.get("q_user").unwrap();
        assert_eq!(account.storage_used_bytes, 40);
    }

    #[test]
    fn test_transaction_commit() {
        let mut storage = InMemoryStorage::new();
        let mut auth = AuthContext::new("tx_user");
        auth.add_role("tx_ns", "writer");
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        storage.create_account(&admin_auth, "tx_user", 1000).unwrap();

        storage.begin_transaction().unwrap();
        storage.set(&auth, "tx_ns", "key1", vec![1]).unwrap();
        storage.set(&auth, "tx_ns", "key2", vec![2]).unwrap();

        // Should not be visible outside transaction yet (if we implemented isolation)
        // But current simple implementation doesn't isolate reads.

        storage.commit_transaction().unwrap();

        // Check values are now permanent
        assert_eq!(storage.get(&auth, "tx_ns", "key1").unwrap(), vec![1]);
        assert_eq!(storage.get(&auth, "tx_ns", "key2").unwrap(), vec![2]);
    }

    #[test]
    fn test_transaction_rollback() {
        let mut storage = InMemoryStorage::new();
        let mut auth = AuthContext::new("tx_user");
        auth.add_role("tx_ns", "writer");
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        storage.create_account(&admin_auth, "tx_user", 1000).unwrap();

        // Set initial value
        storage.set(&auth, "tx_ns", "key1", vec![0]).unwrap();

        storage.begin_transaction().unwrap();
        storage.set(&auth, "tx_ns", "key1", vec![1]).unwrap(); // Modify existing
        storage.set(&auth, "tx_ns", "key2", vec![2]).unwrap(); // Add new

        storage.rollback_transaction().unwrap();

        // key1 should revert to original value
        assert_eq!(storage.get(&auth, "tx_ns", "key1").unwrap(), vec![0]);
        // key2 should not exist
        assert!(matches!(storage.get(&auth, "tx_ns", "key2"), Err(StorageError::NotFound { .. })));
    }

    #[test]
    fn test_audit_log() {
        let mut storage = InMemoryStorage::new();
        let mut auth = AuthContext::new("audit_user");
        auth.add_role("audit_ns", "writer");
        auth.add_role("audit_ns", "admin"); // Needed to view log

        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        storage.create_account(&admin_auth, "audit_user", 1000).unwrap();

        storage.set(&auth, "audit_ns", "log_key", vec![1]).unwrap();
        storage.get(&auth, "audit_ns", "log_key").unwrap(); // This isn't logged currently

        let log = storage.get_audit_log(&auth, Some("audit_ns"), None, 10).unwrap();
        assert_eq!(log.len(), 1); // Only the set is logged
        assert_eq!(log[0].event_type, "write");
        assert_eq!(log[0].user_id, "audit_user");
        assert_eq!(log[0].key, "log_key");

        // Test filtering
        let log_filtered = storage.get_audit_log(&auth, Some("audit_ns"), Some("read"), 10).unwrap();
        assert_eq!(log_filtered.len(), 0);
    }
}
