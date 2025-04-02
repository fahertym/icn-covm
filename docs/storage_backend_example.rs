// storage.rs - Example implementation of persistent storage in ICN-COVM

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use serde::{Serialize, Deserialize};
use std::io::Result as IoResult;

/// Error types for storage operations
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    IoError(String),
    
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    
    #[error("Transaction error: {0}")]
    TransactionError(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("No transaction active")]
    NoTransaction,
}

/// Core trait for storage backends
pub trait StorageBackend {
    /// Retrieve a value from storage
    fn get(&self, key: &str) -> Option<f64>;
    
    /// Store a value in storage
    fn set(&mut self, key: &str, value: f64) -> Result<(), StorageError>;
    
    /// Delete a key from storage
    fn delete(&mut self, key: &str) -> Result<(), StorageError>;
    
    /// Check if a key exists
    fn contains(&self, key: &str) -> bool;
    
    /// List all keys with a given prefix
    fn list_keys(&self, prefix: &str) -> Vec<String>;
    
    /// Begin a transaction
    fn begin_transaction(&mut self) -> Result<(), StorageError>;
    
    /// Commit the current transaction
    fn commit_transaction(&mut self) -> Result<(), StorageError>;
    
    /// Rollback the current transaction
    fn rollback_transaction(&mut self) -> Result<(), StorageError>;
    
    /// Check if a transaction is active
    fn is_transaction_active(&self) -> bool;
}

/// In-memory implementation of StorageBackend for testing and simple use cases
pub struct InMemoryStorage {
    /// Main storage map
    data: HashMap<String, f64>,
    
    /// Transaction staging area
    transaction_data: Option<HashMap<String, Option<f64>>>,
    
    /// Transaction status
    transaction_active: bool,
}

impl InMemoryStorage {
    /// Create a new empty storage instance
    pub fn new() -> Self {
        InMemoryStorage {
            data: HashMap::new(),
            transaction_data: None,
            transaction_active: false,
        }
    }
    
    /// Validate a storage key
    fn validate_key(&self, key: &str) -> Result<(), StorageError> {
        // Keys cannot be empty
        if key.is_empty() {
            return Err(StorageError::InvalidKey("Key cannot be empty".to_string()));
        }
        
        // Keys should use forward slashes for namespaces
        if key.contains('\\') {
            return Err(StorageError::InvalidKey(
                "Keys should use forward slashes, not backslashes".to_string()
            ));
        }
        
        // Keys cannot contain special characters
        let forbidden_chars = ['?', '#', '[', ']', '*'];
        if key.chars().any(|c| forbidden_chars.contains(&c)) {
            return Err(StorageError::InvalidKey(
                format!("Key contains forbidden characters: {}", key)
            ));
        }
        
        Ok(())
    }
}

impl StorageBackend for InMemoryStorage {
    fn get(&self, key: &str) -> Option<f64> {
        if let Err(e) = self.validate_key(key) {
            // Just return None for invalid keys on get operations
            return None;
        }
        
        // If a transaction is active, check the transaction data first
        if self.transaction_active {
            if let Some(tx_data) = &self.transaction_data {
                if let Some(value_opt) = tx_data.get(key) {
                    return match value_opt {
                        Some(value) => Some(*value),  // Modified in transaction
                        None => None,                // Deleted in transaction
                    };
                }
            }
        }
        
        // Fall back to the main data store
        self.data.get(key).copied()
    }
    
    fn set(&mut self, key: &str, value: f64) -> Result<(), StorageError> {
        self.validate_key(key)?;
        
        if self.transaction_active {
            // Store in transaction data
            if let Some(tx_data) = &mut self.transaction_data {
                tx_data.insert(key.to_string(), Some(value));
                Ok(())
            } else {
                Err(StorageError::TransactionError("Transaction data not initialized".to_string()))
            }
        } else {
            // Store directly in main data
            self.data.insert(key.to_string(), value);
            Ok(())
        }
    }
    
    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        self.validate_key(key)?;
        
        if self.transaction_active {
            // Mark as deleted in transaction
            if let Some(tx_data) = &mut self.transaction_data {
                tx_data.insert(key.to_string(), None);
                Ok(())
            } else {
                Err(StorageError::TransactionError("Transaction data not initialized".to_string()))
            }
        } else {
            // Remove directly from main data
            self.data.remove(key);
            Ok(())
        }
    }
    
    fn contains(&self, key: &str) -> bool {
        if let Err(_) = self.validate_key(key) {
            return false;
        }
        
        // If a transaction is active, check the transaction data first
        if self.transaction_active {
            if let Some(tx_data) = &self.transaction_data {
                if let Some(value_opt) = tx_data.get(key) {
                    return value_opt.is_some();  // If None, it's marked for deletion
                }
            }
        }
        
        // Fall back to the main data store
        self.data.contains_key(key)
    }
    
    fn list_keys(&self, prefix: &str) -> Vec<String> {
        // Create a combined view of keys from both main data and transaction
        let mut all_keys = Vec::new();
        
        // Add keys from main data
        for key in self.data.keys() {
            if key.starts_with(prefix) {
                all_keys.push(key.clone());
            }
        }
        
        // If a transaction is active, apply transaction changes
        if self.transaction_active {
            if let Some(tx_data) = &self.transaction_data {
                for (key, value_opt) in tx_data {
                    if key.starts_with(prefix) {
                        match value_opt {
                            Some(_) => {
                                // Add new keys or keep modified keys
                                if !all_keys.contains(key) {
                                    all_keys.push(key.clone());
                                }
                            },
                            None => {
                                // Remove deleted keys
                                all_keys.retain(|k| k != key);
                            }
                        }
                    }
                }
            }
        }
        
        all_keys
    }
    
    fn begin_transaction(&mut self) -> Result<(), StorageError> {
        if self.transaction_active {
            return Err(StorageError::TransactionError("Transaction already active".to_string()));
        }
        
        self.transaction_data = Some(HashMap::new());
        self.transaction_active = true;
        Ok(())
    }
    
    fn commit_transaction(&mut self) -> Result<(), StorageError> {
        if !self.transaction_active {
            return Err(StorageError::NoTransaction);
        }
        
        // Apply all changes from the transaction
        if let Some(tx_data) = &self.transaction_data {
            for (key, value_opt) in tx_data {
                match value_opt {
                    Some(value) => {
                        // Set the value
                        self.data.insert(key.clone(), *value);
                    },
                    None => {
                        // Delete the key
                        self.data.remove(key);
                    }
                }
            }
        }
        
        // Clear the transaction
        self.transaction_data = None;
        self.transaction_active = false;
        
        Ok(())
    }
    
    fn rollback_transaction(&mut self) -> Result<(), StorageError> {
        if !self.transaction_active {
            return Err(StorageError::NoTransaction);
        }
        
        // Just discard the transaction data
        self.transaction_data = None;
        self.transaction_active = false;
        
        Ok(())
    }
    
    fn is_transaction_active(&self) -> bool {
        self.transaction_active
    }
}

/// File-based storage implementation
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct FileStorage {
    /// Main storage data
    data: HashMap<String, f64>,
    
    /// Transaction staging area
    #[serde(skip)]
    transaction_data: Option<HashMap<String, Option<f64>>>,
    
    /// Transaction status
    #[serde(skip)]
    transaction_active: bool,
    
    /// File path for storage
    #[serde(skip)]
    file_path: Option<String>,
}

impl FileStorage {
    /// Create a new file storage instance
    pub fn new(file_path: &str) -> Result<Self, StorageError> {
        let path = Path::new(file_path);
        
        // Load existing data if the file exists
        let mut storage = if path.exists() {
            let contents = fs::read_to_string(path)
                .map_err(|e| StorageError::IoError(e.to_string()))?;
            
            serde_json::from_str(&contents)
                .map_err(|e| StorageError::IoError(format!("Failed to parse storage file: {}", e)))?
        } else {
            FileStorage::default()
        };
        
        storage.file_path = Some(file_path.to_string());
        Ok(storage)
    }
    
    /// Save storage data to file
    fn save(&self) -> Result<(), StorageError> {
        if let Some(file_path) = &self.file_path {
            let json = serde_json::to_string_pretty(&self)
                .map_err(|e| StorageError::IoError(format!("Failed to serialize storage: {}", e)))?;
            
            fs::write(file_path, json)
                .map_err(|e| StorageError::IoError(format!("Failed to write storage file: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Validate a storage key
    fn validate_key(&self, key: &str) -> Result<(), StorageError> {
        // Keys cannot be empty
        if key.is_empty() {
            return Err(StorageError::InvalidKey("Key cannot be empty".to_string()));
        }
        
        // Keys should use forward slashes for namespaces
        if key.contains('\\') {
            return Err(StorageError::InvalidKey(
                "Keys should use forward slashes, not backslashes".to_string()
            ));
        }
        
        // Keys cannot contain special characters
        let forbidden_chars = ['?', '#', '[', ']', '*'];
        if key.chars().any(|c| forbidden_chars.contains(&c)) {
            return Err(StorageError::InvalidKey(
                format!("Key contains forbidden characters: {}", key)
            ));
        }
        
        Ok(())
    }
}

impl StorageBackend for FileStorage {
    fn get(&self, key: &str) -> Option<f64> {
        if let Err(_) = self.validate_key(key) {
            return None;
        }
        
        // If a transaction is active, check the transaction data first
        if self.transaction_active {
            if let Some(tx_data) = &self.transaction_data {
                if let Some(value_opt) = tx_data.get(key) {
                    return match value_opt {
                        Some(value) => Some(*value),  // Modified in transaction
                        None => None,                // Deleted in transaction
                    };
                }
            }
        }
        
        // Fall back to the main data store
        self.data.get(key).copied()
    }
    
    fn set(&mut self, key: &str, value: f64) -> Result<(), StorageError> {
        self.validate_key(key)?;
        
        if self.transaction_active {
            // Store in transaction data
            if let Some(tx_data) = &mut self.transaction_data {
                tx_data.insert(key.to_string(), Some(value));
                Ok(())
            } else {
                Err(StorageError::TransactionError("Transaction data not initialized".to_string()))
            }
        } else {
            // Store directly and save
            self.data.insert(key.to_string(), value);
            self.save()?;
            Ok(())
        }
    }
    
    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        self.validate_key(key)?;
        
        if self.transaction_active {
            // Mark as deleted in transaction
            if let Some(tx_data) = &mut self.transaction_data {
                tx_data.insert(key.to_string(), None);
                Ok(())
            } else {
                Err(StorageError::TransactionError("Transaction data not initialized".to_string()))
            }
        } else {
            // Remove directly and save
            self.data.remove(key);
            self.save()?;
            Ok(())
        }
    }
    
    fn contains(&self, key: &str) -> bool {
        if let Err(_) = self.validate_key(key) {
            return false;
        }
        
        // If a transaction is active, check the transaction data first
        if self.transaction_active {
            if let Some(tx_data) = &self.transaction_data {
                if let Some(value_opt) = tx_data.get(key) {
                    return value_opt.is_some();  // If None, it's marked for deletion
                }
            }
        }
        
        // Fall back to the main data store
        self.data.contains_key(key)
    }
    
    fn list_keys(&self, prefix: &str) -> Vec<String> {
        // Create a combined view of keys from both main data and transaction
        let mut all_keys = Vec::new();
        
        // Add keys from main data
        for key in self.data.keys() {
            if key.starts_with(prefix) {
                all_keys.push(key.clone());
            }
        }
        
        // If a transaction is active, apply transaction changes
        if self.transaction_active {
            if let Some(tx_data) = &self.transaction_data {
                for (key, value_opt) in tx_data {
                    if key.starts_with(prefix) {
                        match value_opt {
                            Some(_) => {
                                // Add new keys or keep modified keys
                                if !all_keys.contains(key) {
                                    all_keys.push(key.clone());
                                }
                            },
                            None => {
                                // Remove deleted keys
                                all_keys.retain(|k| k != key);
                            }
                        }
                    }
                }
            }
        }
        
        all_keys
    }
    
    fn begin_transaction(&mut self) -> Result<(), StorageError> {
        if self.transaction_active {
            return Err(StorageError::TransactionError("Transaction already active".to_string()));
        }
        
        self.transaction_data = Some(HashMap::new());
        self.transaction_active = true;
        Ok(())
    }
    
    fn commit_transaction(&mut self) -> Result<(), StorageError> {
        if !self.transaction_active {
            return Err(StorageError::NoTransaction);
        }
        
        // Apply all changes from the transaction
        if let Some(tx_data) = &self.transaction_data {
            for (key, value_opt) in tx_data {
                match value_opt {
                    Some(value) => {
                        // Set the value
                        self.data.insert(key.clone(), *value);
                    },
                    None => {
                        // Delete the key
                        self.data.remove(key);
                    }
                }
            }
        }
        
        // Save changes to disk
        self.save()?;
        
        // Clear the transaction
        self.transaction_data = None;
        self.transaction_active = false;
        
        Ok(())
    }
    
    fn rollback_transaction(&mut self) -> Result<(), StorageError> {
        if !self.transaction_active {
            return Err(StorageError::NoTransaction);
        }
        
        // Just discard the transaction data
        self.transaction_data = None;
        self.transaction_active = false;
        
        Ok(())
    }
    
    fn is_transaction_active(&self) -> bool {
        self.transaction_active
    }
}

// VM Operation additions
impl crate::vm::VM {
    /// StoreP operation: Store a value in persistent storage
    pub fn op_storep(&mut self, key: &str) -> Result<(), crate::vm::VMError> {
        // Validate caller has permission to write to this key
        if let Some(auth) = &self.auth_context {
            self.check_storage_permission(key, true)?;
        }
        
        let value = self.pop_one("StoreP")?;
        
        // Perform the storage operation
        self.storage.set(key, value)
            .map_err(|e| crate::vm::VMError::StorageError(e.to_string()))
    }
    
    /// LoadP operation: Load a value from persistent storage
    pub fn op_loadp(&mut self, key: &str) -> Result<(), crate::vm::VMError> {
        // Validate caller has permission to read this key
        if let Some(auth) = &self.auth_context {
            self.check_storage_permission(key, false)?;
        }
        
        // Perform the storage operation
        match self.storage.get(key) {
            Some(value) => {
                self.stack.push(value);
                Ok(())
            },
            None => Err(crate::vm::VMError::VariableNotFound(key.to_string())),
        }
    }
    
    /// BeginTx operation: Begin a storage transaction
    pub fn op_begintx(&mut self) -> Result<(), crate::vm::VMError> {
        // Start a transaction
        self.storage.begin_transaction()
            .map_err(|e| crate::vm::VMError::StorageError(e.to_string()))
    }
    
    /// CommitTx operation: Commit a storage transaction
    pub fn op_committx(&mut self) -> Result<(), crate::vm::VMError> {
        // Commit the transaction
        self.storage.commit_transaction()
            .map_err(|e| crate::vm::VMError::StorageError(e.to_string()))
    }
    
    /// RollbackTx operation: Rollback a storage transaction
    pub fn op_rollbacktx(&mut self) -> Result<(), crate::vm::VMError> {
        // Rollback the transaction
        self.storage.rollback_transaction()
            .map_err(|e| crate::vm::VMError::StorageError(e.to_string()))
    }
    
    /// Check if the caller has permission to access a storage key
    fn check_storage_permission(&self, key: &str, write: bool) -> Result<(), crate::vm::VMError> {
        // This is a placeholder for the actual permission check
        // In a real implementation, this would check the caller's roles against
        // a permission system specific to storage namespaces
        
        if let Some(auth) = &self.auth_context {
            // Example: System admin can access any key
            if auth.caller.roles.contains(&"system_admin".to_string()) {
                return Ok(());
            }
            
            // Example: Users can only access their own namespace
            if key.starts_with(&format!("member/{}/", auth.caller.id)) {
                return Ok(());
            }
            
            // Example: Organization admins can access their org's namespace
            if auth.caller.roles.iter().any(|r| r.starts_with("org_admin_")) {
                // Extract org IDs from roles like "org_admin_acme"
                let org_ids: Vec<&str> = auth.caller.roles.iter()
                    .filter(|r| r.starts_with("org_admin_"))
                    .map(|r| &r[10..])
                    .collect();
                
                // Check if the key starts with any of the user's orgs
                for org_id in org_ids {
                    if key.starts_with(&format!("org/{}/", org_id)) {
                        return Ok(());
                    }
                }
            }
            
            // If no conditions matched and we need write access, deny
            if write {
                return Err(crate::vm::VMError::PermissionDenied(format!(
                    "Caller {} does not have write permission for key {}",
                    auth.caller.id, key
                )));
            }
        }
        
        // If no auth context or read-only access, allow
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_in_memory_storage_basic() {
        let mut storage = InMemoryStorage::new();
        
        // Test basic operations
        assert_eq!(storage.get("test"), None);
        assert_eq!(storage.contains("test"), false);
        
        storage.set("test", 42.0).unwrap();
        assert_eq!(storage.get("test"), Some(42.0));
        assert_eq!(storage.contains("test"), true);
        
        storage.delete("test").unwrap();
        assert_eq!(storage.get("test"), None);
        assert_eq!(storage.contains("test"), false);
    }
    
    #[test]
    fn test_in_memory_storage_transactions() {
        let mut storage = InMemoryStorage::new();
        
        // Set up initial data
        storage.set("a", 1.0).unwrap();
        storage.set("b", 2.0).unwrap();
        
        // Start a transaction
        storage.begin_transaction().unwrap();
        
        // Make changes in the transaction
        storage.set("a", 10.0).unwrap();
        storage.set("c", 3.0).unwrap();
        storage.delete("b").unwrap();
        
        // Verify changes are visible within the transaction
        assert_eq!(storage.get("a"), Some(10.0));
        assert_eq!(storage.get("b"), None);
        assert_eq!(storage.get("c"), Some(3.0));
        
        // Rollback the transaction
        storage.rollback_transaction().unwrap();
        
        // Verify original data is restored
        assert_eq!(storage.get("a"), Some(1.0));
        assert_eq!(storage.get("b"), Some(2.0));
        assert_eq!(storage.get("c"), None);
        
        // Try another transaction and commit it
        storage.begin_transaction().unwrap();
        storage.set("a", 100.0).unwrap();
        storage.set("d", 4.0).unwrap();
        storage.commit_transaction().unwrap();
        
        // Verify changes are persisted
        assert_eq!(storage.get("a"), Some(100.0));
        assert_eq!(storage.get("b"), Some(2.0));
        assert_eq!(storage.get("c"), None);
        assert_eq!(storage.get("d"), Some(4.0));
    }
    
    #[test]
    fn test_namespace_listing() {
        let mut storage = InMemoryStorage::new();
        
        // Create keys in different namespaces
        storage.set("org/acme/user/alice", 1.0).unwrap();
        storage.set("org/acme/user/bob", 2.0).unwrap();
        storage.set("org/beta/user/charlie", 3.0).unwrap();
        
        // List keys by namespace
        let acme_keys = storage.list_keys("org/acme/");
        assert_eq!(acme_keys.len(), 2);
        assert!(acme_keys.contains(&"org/acme/user/alice".to_string()));
        assert!(acme_keys.contains(&"org/acme/user/bob".to_string()));
        
        let beta_keys = storage.list_keys("org/beta/");
        assert_eq!(beta_keys.len(), 1);
        assert!(beta_keys.contains(&"org/beta/user/charlie".to_string()));
        
        // Test with transactions
        storage.begin_transaction().unwrap();
        storage.set("org/acme/user/dave", 4.0).unwrap();
        storage.delete("org/acme/user/alice").unwrap();
        
        let tx_acme_keys = storage.list_keys("org/acme/");
        assert_eq!(tx_acme_keys.len(), 2);
        assert!(!tx_acme_keys.contains(&"org/acme/user/alice".to_string()));
        assert!(tx_acme_keys.contains(&"org/acme/user/bob".to_string()));
        assert!(tx_acme_keys.contains(&"org/acme/user/dave".to_string()));
    }
} 