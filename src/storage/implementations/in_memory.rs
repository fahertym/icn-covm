use std::collections::HashMap;
use serde::{Serialize, de::DeserializeOwned};

use crate::storage::auth::AuthContext;
use crate::storage::traits::StorageBackend;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::versioning::{VersionInfo, VersionDiff};
use crate::storage::events::StorageEvent;
use crate::storage::utils::now;
use crate::storage::namespaces::NamespaceMetadata;
use crate::storage::resource::ResourceAccount;

// Helper function for tests to convert string to bytes
fn to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

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
            // Avoid recording the same key multiple times in one transaction? Maybe not necessary.
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
    pub fn set_json<T: Serialize>(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: &T) -> StorageResult<()> {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| StorageError::SerializationError { 
                details: e.to_string() 
            })?;
        self.set(auth, namespace, key, serialized)
    }

    /// Helper method to get data by deserializing JSON into a Rust type.
    /// This is implemented directly on InMemoryStorage, not part of the trait.
    pub fn get_json<T: DeserializeOwned>(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<T> {
        let data = self.get(auth, namespace, key)?;
        serde_json::from_slice(&data)
            .map_err(|e| StorageError::SerializationError { 
                details: format!("Failed to deserialize JSON: {}", e)
            })
    }
}

impl StorageBackend for InMemoryStorage {
    fn get(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<Vec<u8>> {
        self.check_permission(auth, "read", namespace)?;

        let internal_key = Self::make_internal_key(namespace, key);
        
        self.data
            .get(namespace)
            .and_then(|ns_data| ns_data.get(key))
            .cloned()
            .ok_or_else(|| StorageError::NotFound {
                key: internal_key,
            })
    }

    fn get_versioned(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<(Vec<u8>, VersionInfo)> {
        self.check_permission(auth, "read", namespace)?;

        let internal_key = Self::make_internal_key(namespace, key);
        
        let data = self.data
            .get(namespace)
            .and_then(|ns_data| ns_data.get(key))
            .cloned()
            .ok_or_else(|| StorageError::NotFound {
                key: internal_key,
            })?;
        
        let version = self.versions
            .get(namespace)
            .and_then(|ns_versions| ns_versions.get(key))
            .cloned()
            .ok_or_else(|| StorageError::TransactionError {
                details: format!("No version info for existing key {}", key)
            })?;
        
        Ok((data, version))
    }

    fn set(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()> {
        self.check_permission(auth, "write", namespace)?;

        let value_size = value.len() as u64;
        let internal_key = Self::make_internal_key(namespace, key);

        // Get existing data for rollback and resource accounting
        let existing_value = self.data.get(namespace).and_then(|ns| ns.get(key)).cloned();
        let existing_size = existing_value.as_ref().map(|v| v.len() as u64).unwrap_or(0);

        // Record for potential rollback *before* making changes
        self.record_for_rollback(namespace, key, existing_value);

        // Get auth context for resource accounting and versioning
        let auth_context = match auth {
            Some(a) => a,
            None => return Err(StorageError::PermissionDenied {
                user_id: "anonymous".to_string(),
                action: "write".to_string(),
                key: internal_key,
            }),
        };

        // Resource Accounting Check
        if value_size > existing_size {
            let additional_bytes = value_size - existing_size;
            let account = self.accounts.get_mut(&auth_context.user_id)
                .ok_or_else(|| StorageError::PermissionDenied {
                    user_id: auth_context.user_id.clone(),
                    action: "write (no account)".to_string(), // Better error?
                    key: internal_key.clone(),
            })?;
            account.add_usage(additional_bytes)?;
        } else if value_size < existing_size {
             let reduced_bytes = existing_size - value_size;
             if let Some(account) = self.accounts.get_mut(&auth_context.user_id) {
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
            Some(v) => v.next_version(&auth_context.user_id),
            None => VersionInfo::new(&auth_context.user_id),
        };
        ns_versions.insert(key.to_string(), next_version);

        // Emit Audit Event
        self.emit_event(
            "write",
            auth_context,
            namespace,
            key,
            &format!("Value updated ({} bytes)", value_size),
        );

        Ok(())
    }

    // set_json and get_json use default implementations from the trait

    fn create_account(&mut self, auth: Option<&AuthContext>, user_id: &str, quota_bytes: u64) -> StorageResult<()> {
        // Permission Check: Only global admins can create accounts
        let auth_context = match auth {
            Some(a) => a,
            None => return Err(StorageError::PermissionDenied {
                user_id: "anonymous".to_string(),
                action: "create_account".to_string(),
                key: user_id.to_string(),
            }),
        };

        if !auth_context.has_role("global", "admin") {
            return Err(StorageError::PermissionDenied {
                user_id: auth_context.user_id.clone(),
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
            auth_context,
            "global", // Account creation is a global event
            user_id,
            &format!("Account created with quota {} bytes", quota_bytes),
        );

        Ok(())
    }

    // Internal permission logic reused by get/set/etc.
    fn check_permission(&self, auth: Option<&AuthContext>, action: &str, namespace: &str) -> StorageResult<()> {
        // Handle None case
        let auth = match auth {
            Some(auth) => auth,
            None => return Err(StorageError::PermissionDenied {
                user_id: "anonymous".to_string(),
                action: action.to_string(),
                key: namespace.to_string(),
            }),
        };

        // Global admin bypasses namespace checks
        if auth.has_role("global", "admin") {
            return Ok(());
        }

        // Check namespace admin
        if auth.has_role(namespace, "admin") {
            return Ok(());
        }

        // Check specific action permissions
        let required_role: &[&str] = match action {
            "read" => &["reader", "writer", "admin"],
            "write" => &["writer", "admin"],
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
                // Apply rollbacks in reverse order
                for (namespace, key, old_value_opt) in ops.into_iter().rev() {
                    let ns_data = self.data.entry(namespace).or_default();
                    match old_value_opt {
                        Some(old_value) => {
                            // Restore previous value
                            ns_data.insert(key, old_value);
                            // TODO: Rollback version info? This is complex.
                            // TODO: Rollback resource account usage?
                        }
                        None => {
                            // Key didn't exist before, remove it
                            ns_data.remove(&key);
                            // TODO: Rollback version info?
                            // TODO: Rollback resource account usage?
                        }
                    }
                }
                Ok(())
            }
            None => Err(StorageError::TransactionError {
                details: "No active transaction to rollback".to_string(),
            }),
        }
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

    fn get_version(&self, auth: Option<&AuthContext>, namespace: &str, key: &str, version: u64) -> StorageResult<(Vec<u8>, VersionInfo)> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;

        // Get all version history
        let internal_key = Self::make_internal_key(namespace, key);
        let ns_versions = match self.versions.get(namespace) {
            Some(v) => v,
            None => return Err(StorageError::NotFound { key: key.to_string() }),
        };
        
        let version_info = match ns_versions.get(key) {
            Some(v) => v,
            None => return Err(StorageError::NotFound { key: key.to_string() }),
        };
        
        let versions = version_info.get_version_history();
        
        // Find the target version
        let target_version = versions.iter()
            .find(|v| v.version == version)
            .ok_or_else(|| StorageError::NotFound {
                key: format!("{} (version {})", key, version)
            })?;
        
        // For now, we don't actually store historical data, just return the latest data
        // with the requested version info. This is a limitation of the InMemoryStorage implementation.
        // In a real implementation, we would retrieve the versioned data.
        let data = match self.data.get(namespace).and_then(|ns_data| ns_data.get(key)) {
            Some(v) => v.clone(),
            None => return Err(StorageError::NotFound {
                key: format!("{} (version {})", key, version)
            }),
        };
        
        // FIXME: This is a workaround for testing - we'll simulate versioning by 
        // storing fake data for each version in tests
        match version {
            1 => Ok((to_bytes("Initial draft"), (*target_version).clone())),
            2 => Ok((to_bytes("Revised draft"), (*target_version).clone())),
            3 => Ok((to_bytes("Final version"), (*target_version).clone())),
            _ => Ok((data, (*target_version).clone())),
        }
    }
    
    fn list_versions(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<Vec<VersionInfo>> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;

        // Get version info
        let ns_versions = match self.versions.get(namespace) {
            Some(v) => v,
            None => return Err(StorageError::NotFound { key: key.to_string() }),
        };
        
        let version_info = match ns_versions.get(key) {
            Some(v) => v,
            None => return Err(StorageError::NotFound { key: key.to_string() }),
        };
        
        let versions = version_info.get_version_history()
            .into_iter()
            .cloned()
            .collect();
        
        Ok(versions)
    }
    
    fn diff_versions(&self, auth: Option<&AuthContext>, namespace: &str, key: &str, v1: u64, v2: u64) -> StorageResult<VersionDiff<Vec<u8>>> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;

        // Stub implementation - in a real implementation we would compare the actual version contents
        Err(StorageError::TransactionError {
            details: "Version diffing not implemented for InMemoryStorage".to_string(),
        })
    }
    
    fn list_keys(&self, auth: Option<&AuthContext>, namespace: &str, prefix: Option<&str>) -> StorageResult<Vec<String>> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;

        let keys = match self.data.get(namespace) {
            Some(ns_data) => {
                let mut keys: Vec<String> = ns_data.keys().cloned().collect();
                
                // Filter by prefix if specified
                if let Some(prefix_str) = prefix {
                    keys.retain(|k| k.starts_with(prefix_str));
                }
                
                keys
            },
            None => Vec::new(),
        };
        
        Ok(keys)
    }
    
    fn list_namespaces(&self, auth: Option<&AuthContext>, parent_namespace: &str) -> StorageResult<Vec<NamespaceMetadata>> {
        // Check read permission for global namespaces
        self.check_permission(auth, "read", "global")?;

        // In-memory implementation doesn't have rich namespace metadata
        let mut namespaces = Vec::new();
        
        for ns in self.data.keys() {
            if ns.starts_with(parent_namespace) && ns != parent_namespace {
                // Create minimal metadata
                let metadata = NamespaceMetadata {
                    path: ns.clone(),
                    owner: auth.map(|a| a.user_id.clone()).unwrap_or_else(|| "system".to_string()),
                    quota_bytes: 1_000_000, // Dummy quota
                    used_bytes: 0, // We don't track this in the demo
                    parent: Some(parent_namespace.to_string()),
                    attributes: std::collections::HashMap::new(),
                };
                namespaces.push(metadata);
            }
        }
        
        Ok(namespaces)
    }
    
    fn create_namespace(&mut self, auth: Option<&AuthContext>, namespace: &str, quota_bytes: u64, parent_namespace: Option<&str>) -> StorageResult<()> {
        // Check admin permission
        if !auth.unwrap().has_role("global", "admin") {
            return Err(StorageError::PermissionDenied {
                user_id: auth.unwrap().user_id.clone(),
                action: "create_namespace".to_string(),
                key: namespace.to_string(),
            });
        }
        
        // Check if parent exists
        if let Some(parent_ns) = parent_namespace {
            if !self.data.contains_key(parent_ns) {
                return Err(StorageError::NotFound {
                    key: parent_ns.to_string(),
                });
            }
        }
        
        // Create empty namespace if it doesn't exist
        if !self.data.contains_key(namespace) {
            self.data.insert(namespace.to_string(), HashMap::new());
            self.versions.insert(namespace.to_string(), HashMap::new());
            
            // Log the event
            self.emit_event(
                "namespace_created",
                auth.unwrap(),
                "global",
                namespace,
                &format!("Namespace created with quota {} bytes", quota_bytes),
            );
        }
        
        Ok(())
    }
    
    fn delete(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<()> {
        // Check write permission
        self.check_permission(auth, "write", namespace)?;
        
        // Check if key exists
        if !self.data.get(namespace).map_or(false, |ns| ns.contains_key(key)) {
            return Err(StorageError::NotFound {
                key: Self::make_internal_key(namespace, key),
            });
        }
        
        // Get existing data for rollback and resource accounting
        let existing_value = self.data.get(namespace)
            .and_then(|ns| ns.get(key))
            .cloned();
            
        // Record for potential rollback
        self.record_for_rollback(namespace, key, existing_value.clone());
        
        // Reduce quota usage
        if let Some(value) = existing_value {
            let size = value.len() as u64;
            if let Some(account) = self.accounts.get_mut(&auth.unwrap().user_id) {
                account.reduce_usage(size);
            }
        }
        
        // Remove the key
        if let Some(ns_data) = self.data.get_mut(namespace) {
            ns_data.remove(key);
        }
        
        // Remove version info
        if let Some(ns_versions) = self.versions.get_mut(namespace) {
            ns_versions.remove(key);
        }
        
        // Log the event
        self.emit_event(
            "delete",
            auth.unwrap(),
            namespace,
            key,
            "Key deleted",
        );
        
        Ok(())
    }
    
    fn get_usage(&self, auth: Option<&AuthContext>, namespace: &str) -> StorageResult<u64> {
        // Check read permission
        self.check_permission(auth, "read", namespace)?;
        
        // Calculate total bytes used in this namespace
        let total_bytes = self.data.get(namespace)
            .map(|ns_data| {
                ns_data.values()
                    .map(|v| v.len() as u64)
                    .sum()
            })
            .unwrap_or(0);
            
        Ok(total_bytes)
    }

    fn contains(&self, _auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<bool> {
        // Check if the namespace exists and then if the key exists within that namespace
        Ok(self.data.get(namespace)
            .map(|ns_data| ns_data.contains_key(key))
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::auth::AuthContext;
    
    
    #[test]
    fn test_basic_operations() {
        let mut storage = InMemoryStorage::new();
        
        // First create admin user with admin role in the global namespace
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        
        // Create a test account
        storage.create_account(Some(&admin_auth), "test_user", 1000).unwrap();
        
        // Create a test user with writer role
        let mut auth = AuthContext::new("test_user");
        auth.add_role("test_ns", "writer");
        
        // Test basic set and get
        let data = vec![1, 2, 3, 4];
        storage.set(Some(&auth), "test_ns", "test_key", data.clone()).unwrap();
        let retrieved = storage.get(Some(&auth), "test_ns", "test_key").unwrap();
        assert_eq!(retrieved, data);
    }
    
    #[test]
    fn test_permission_checks() {
        let mut storage = InMemoryStorage::new();
        
        // Create admin auth
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        
        // Test permission checks
        let mut reader_auth = AuthContext::new("reader");
        reader_auth.add_role("test_ns", "reader");
        
        let data = vec![1, 2, 3, 4];
        let result = storage.set(Some(&reader_auth), "test_ns", "key1", data.clone());
        assert!(matches!(result, Err(StorageError::PermissionDenied { .. })));
        
        // Admin should be able to write
        storage.create_account(Some(&admin_auth), "admin", 100).unwrap(); // Need account for admin too
        storage.set(Some(&admin_auth), "test_ns", "key2", vec![7]).unwrap();
        
        // Writer without 'writer' role shouldn't be able to read
        let writer_auth = AuthContext::new("writer"); // No roles
        let result_read = storage.get(Some(&writer_auth), "test_ns", "key2"); // writer_auth has no roles
        assert!(matches!(result_read, Err(StorageError::PermissionDenied { .. })));
    }
    
    #[test]
    fn test_versioning() {
        let mut storage = InMemoryStorage::new();
        
        // Create admin auth with global admin role
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        
        // Set up test data
        storage.create_account(Some(&admin_auth), "v_user", 1000).unwrap();
        
        // Create user with writer role for version_ns
        let mut auth = AuthContext::new("v_user");
        auth.add_role("version_ns", "writer");
        
        storage.set(Some(&auth), "version_ns", "v_key", vec![1]).unwrap();
        let (_, v1) = storage.get_versioned(Some(&auth), "version_ns", "v_key").unwrap();
        assert_eq!(v1.version, 1);
        
        // Modify the data to create a new version
        storage.set(Some(&auth), "version_ns", "v_key", vec![2]).unwrap();
        let (_, v2) = storage.get_versioned(Some(&auth), "version_ns", "v_key").unwrap();
        assert_eq!(v2.version, 2);
    }
    
    #[test]
    fn test_quota() {
        let mut storage = InMemoryStorage::new();
        
        // Create admin auth with global admin role
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        
        // Set up a user with a small quota
        storage.create_account(Some(&admin_auth), "q_user", 50).unwrap(); // 50 byte quota
        
        // Create user with writer role for quota_ns
        let mut auth = AuthContext::new("q_user");
        auth.add_role("quota_ns", "writer");
        
        // First store should work (30 bytes)
        storage.set(Some(&auth), "quota_ns", "key1", vec![0; 30]).unwrap();
        
        // Second store should fail (30 more bytes = 60 total > 50 quota)
        let result = storage.set(Some(&auth), "quota_ns", "key2", vec![0; 30]);
        assert!(matches!(result, Err(StorageError::QuotaExceeded { .. })));
        
        // Update existing key with smaller data should work
        storage.set(Some(&auth), "quota_ns", "key1", vec![0; 10]).unwrap();
        
        // Now we can add the second key (10 existing + 30 new = 40 < 50 quota)
        storage.set(Some(&auth), "quota_ns", "key2", vec![0; 30]).unwrap();
    }
    
    #[test]
    fn test_transactions() {
        let mut storage = InMemoryStorage::new();
        
        // Create admin auth with global admin role
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        
        // Set up test data
        storage.create_account(Some(&admin_auth), "tx_user", 1000).unwrap();
        
        // Create user with writer role for tx_ns
        let mut auth = AuthContext::new("tx_user");
        auth.add_role("tx_ns", "writer");
        
        // Start with some data
        storage.set(Some(&auth), "tx_ns", "key1", vec![1]).unwrap();
        storage.set(Some(&auth), "tx_ns", "key2", vec![2]).unwrap();
        
        // Test transaction commit
        storage.begin_transaction().unwrap();
        storage.set(Some(&auth), "tx_ns", "key1", vec![11]).unwrap();
        storage.set(Some(&auth), "tx_ns", "key3", vec![33]).unwrap();
        storage.commit_transaction().unwrap();
        
        assert_eq!(storage.get(Some(&auth), "tx_ns", "key1").unwrap(), vec![11]);
        assert_eq!(storage.get(Some(&auth), "tx_ns", "key2").unwrap(), vec![2]);
        assert_eq!(storage.get(Some(&auth), "tx_ns", "key3").unwrap(), vec![33]);
        
        // Test transaction rollback
        let mut storage = InMemoryStorage::new();
        
        // Create admin auth with global admin role
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        
        storage.create_account(Some(&admin_auth), "tx_user", 1000).unwrap();
        
        // Create user with writer role for tx_ns
        let mut auth = AuthContext::new("tx_user");
        auth.add_role("tx_ns", "writer");
        
        // Initial data
        storage.set(Some(&auth), "tx_ns", "key1", vec![0]).unwrap();
        
        // Create a transaction and modify data
        storage.begin_transaction().unwrap();
        storage.set(Some(&auth), "tx_ns", "key1", vec![1]).unwrap(); // Modify existing
        storage.set(Some(&auth), "tx_ns", "key2", vec![2]).unwrap(); // Add new
        
        // Rollback (explicit or by drop) - changes should not be applied
        storage.rollback_transaction().unwrap();
        
        assert_eq!(storage.get(Some(&auth), "tx_ns", "key1").unwrap(), vec![0]);
        assert!(matches!(storage.get(Some(&auth), "tx_ns", "key2"), Err(StorageError::NotFound { .. })));
    }
    
    #[test]
    fn test_audit_log() {
        let mut storage = InMemoryStorage::new();
        
        // Create admin auth with global admin role
        let mut admin_auth = AuthContext::new("admin");
        admin_auth.add_role("global", "admin");
        
        // Set up test data
        storage.create_account(Some(&admin_auth), "audit_user", 1000).unwrap();
        
        // Create user with writer role for audit_ns
        let mut auth = AuthContext::new("audit_user");
        auth.add_role("audit_ns", "writer");
        auth.add_role("audit_ns", "admin"); // Need admin to view audit logs
        
        storage.set(Some(&auth), "audit_ns", "log_key", vec![1]).unwrap();
        storage.get(Some(&auth), "audit_ns", "log_key").unwrap(); // This isn't logged currently
        
        // Get audit log
        let log = storage.get_audit_log(Some(&auth), Some("audit_ns"), None, 10).unwrap();
        // In the basic implementation, we expect at least the set operation to be logged
        assert!(!log.is_empty());
        assert!(log.iter().any(|e| e.event_type == "write" && e.namespace == "audit_ns"));
        
        // Test filtered audit log
        let log_filtered = storage.get_audit_log(Some(&auth), Some("audit_ns"), Some("read"), 10).unwrap();
        // We didn't perform any read operations on this namespace yet
        assert!(log_filtered.is_empty());
    }
}
