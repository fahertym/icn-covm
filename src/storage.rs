use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use serde::{Serialize, Deserialize};

/// Type alias for a timestamp (milliseconds since Unix epoch)
pub type Timestamp = u64;

/// Get the current timestamp
pub fn now() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Authentication context for storage operations
///
/// This struct contains information about the caller and their permissions,
/// allowing the storage backend to enforce access control policies.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthContext {
    /// The ID of the caller
    pub caller: String,
    
    /// Roles associated with the caller
    pub roles: Vec<String>,
    
    /// Timestamp of the request
    pub timestamp: Timestamp,
    
    /// Optional delegation chain (for liquid democracy)
    pub delegation_chain: Vec<String>,
}

impl AuthContext {
    /// Create a new authentication context
    pub fn new(caller: &str) -> Self {
        Self {
            caller: caller.to_string(),
            roles: Vec::new(),
            timestamp: now(),
            delegation_chain: Vec::new(),
        }
    }
    
    /// Create a new authentication context with roles
    pub fn with_roles(caller: &str, roles: Vec<String>) -> Self {
        Self {
            caller: caller.to_string(),
            roles,
            timestamp: now(),
            delegation_chain: Vec::new(),
        }
    }
    
    /// Check if the caller has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
    
    /// Add a role to the caller
    pub fn add_role(&mut self, role: &str) {
        if !self.has_role(role) {
            self.roles.push(role.to_string());
        }
    }
}

/// Resource account for tracking storage usage
///
/// This struct tracks storage usage and enforces quotas for cooperative
/// resource accounting and economic operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceAccount {
    /// Account identifier
    pub id: String,
    
    /// Current resource balance
    pub balance: f64,
    
    /// Maximum allowed usage (quota)
    pub quota: f64,
    
    /// Usage history for auditing
    pub usage_history: Vec<(Timestamp, f64, String)>,
}

impl ResourceAccount {
    /// Create a new resource account with the given quota
    pub fn new(id: &str, quota: f64) -> Self {
        Self {
            id: id.to_string(),
            balance: quota,
            quota,
            usage_history: Vec::new(),
        }
    }
    
    /// Deduct resources and record the operation
    pub fn deduct(&mut self, amount: f64, operation: &str) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            let mut new_history = self.usage_history.clone();
            new_history.push((now(), amount, operation.to_string()));
            self.usage_history = new_history;
            true
        } else {
            false
        }
    }
    
    /// Reset balance to quota
    pub fn reset(&mut self) {
        self.balance = self.quota;
    }
}

/// Version information for stored data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Version number (increments with each update)
    pub version: usize,
    
    /// Timestamp of when this version was created
    pub timestamp: Timestamp,
    
    /// ID of the user who created this version
    pub author: String,
    
    /// Optional comment about this version
    pub comment: Option<String>,
}

/// Error variants that can occur during storage operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum StorageError {
    /// Key not found in storage
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    /// Error accessing the storage backend
    #[error("Storage access error: {0}")]
    AccessError(String),
    
    /// Error during serialization or deserialization
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Permission denied for the requested operation
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    /// Transaction-related error
    #[error("Transaction error: {0}")]
    TransactionError(String),
    
    /// Resource quota exceeded
    #[error("Resource quota exceeded: {0}")]
    QuotaExceeded(String),
    
    /// Version not found error
    #[error("Version not found: {0}")]
    VersionNotFound(String),
    
    /// Federation or synchronization error 
    #[error("Federation error: {0}")]
    FederationError(String),
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage event for audit logging
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StorageEvent {
    /// Access to a storage key
    Access {
        /// The key being accessed
        key: String,
        
        /// The type of access (get, set, delete)
        action: String,
        
        /// The ID of the user performing the action
        user: String,
        
        /// The timestamp of the action
        timestamp: Timestamp,
    },
    
    /// Transaction operation
    Transaction {
        /// The type of transaction operation (begin, commit, rollback)
        action: String,
        
        /// The ID of the user performing the action
        user: String,
        
        /// The timestamp of the action
        timestamp: Timestamp,
    },
    
    /// Resource usage event
    ResourceUsage {
        /// The account being charged
        account: String,
        
        /// The amount of resources used
        amount: f64,
        
        /// The operation being performed
        operation: String,
        
        /// The timestamp of the action
        timestamp: Timestamp,
    },
}

/// Governance namespace helper for storage keys
pub struct GovernanceNamespace;

impl GovernanceNamespace {
    /// Create a key in the governance namespace
    pub const fn governance(key: &str) -> String {
        format!("governance/{}", key)
    }
    
    /// Create a key in the votes namespace
    pub const fn votes(proposal_id: &str, voter_id: &str) -> String {
        format!("governance/votes/{}/{}", proposal_id, voter_id)
    }
    
    /// Create a key in the delegations namespace
    pub const fn delegations(from: &str, to: &str) -> String {
        format!("governance/delegations/{}/{}", from, to)
    }
    
    /// Create a key in the proposals namespace
    pub const fn proposals(proposal_id: &str) -> String {
        format!("governance/proposals/{}", proposal_id)
    }
    
    /// Create a key in the members namespace
    pub const fn members(member_id: &str) -> String {
        format!("governance/members/{}", member_id)
    }
    
    /// Create a key in the config namespace
    pub const fn config(key: &str) -> String {
        format!("governance/config/{}", key)
    }
}

/// Trait defining the interface for storage backends
///
/// This trait provides a common interface for different storage implementations,
/// allowing the VM to persist data across runs using various storage technologies.
/// The storage backend is designed with cooperative governance in mind, providing
/// namespaces for governance data, role-based access control, transaction support,
/// and audit logging.
pub trait StorageBackend {
    /// Get a value from storage by key
    fn get(&self, key: &str) -> StorageResult<String>;
    
    /// Set a value in storage by key
    fn set(&mut self, key: &str, value: &str) -> StorageResult<()>;
    
    /// Delete a value from storage by key
    fn delete(&mut self, key: &str) -> StorageResult<()>;
    
    /// Check if a key exists in storage
    fn contains(&self, key: &str) -> bool;
    
    /// List all keys in storage (optionally filtered by prefix)
    fn list_keys(&self, prefix: Option<&str>) -> Vec<String>;
    
    /// Begin a transaction
    fn begin_transaction(&mut self) -> StorageResult<()>;
    
    /// Commit the current transaction
    fn commit_transaction(&mut self) -> StorageResult<()>;
    
    /// Rollback the current transaction
    fn rollback_transaction(&mut self) -> StorageResult<()>;
    
    /// Get a value with authorization check
    fn get_with_auth(&self, auth: &AuthContext, key: &str) -> StorageResult<String> {
        // By default, just call get
        // Implementations should override this to add proper authorization checks
        self.get(key)
    }
    
    /// Set a value with authorization check
    fn set_with_auth(&mut self, auth: &AuthContext, key: &str, value: &str) -> StorageResult<()> {
        // By default, just call set
        // Implementations should override this to add proper authorization checks
        self.set(key, value)
    }
    
    /// Delete a value with authorization check
    fn delete_with_auth(&mut self, auth: &AuthContext, key: &str) -> StorageResult<()> {
        // By default, just call delete
        // Implementations should override this to add proper authorization checks
        self.delete(key)
    }
    
    /// Set a value with resource accounting
    fn set_with_resources(
        &mut self,
        auth: &AuthContext,
        key: &str,
        value: &str,
        account: &mut ResourceAccount,
    ) -> StorageResult<()> {
        // Calculate resource cost (default: 1 unit per KB)
        let cost = (value.len() as f64) / 1024.0;
        
        // Check if the account has sufficient resources
        if !account.deduct(cost, &format!("set:{}", key)) {
            return Err(StorageError::QuotaExceeded(format!(
                "Insufficient resources to store {} (cost: {}, available: {})",
                key, cost, account.balance
            )));
        }
        
        // Store the value
        self.set_with_auth(auth, key, value)
    }
    
    /// Get a versioned value (default: not supported)
    fn get_versioned(&self, key: &str, version: usize) -> StorageResult<String> {
        // Default implementation: not supported
        Err(StorageError::VersionNotFound(format!(
            "Versioning not supported for key {}, version {}",
            key, version
        )))
    }
    
    /// List all versions of a key (default: not supported)
    fn list_versions(&self, key: &str) -> StorageResult<Vec<VersionInfo>> {
        // Default implementation: not supported
        Err(StorageError::VersionNotFound(format!(
            "Versioning not supported for key {}",
            key
        )))
    }
    
    /// Emit a storage event for audit logging (default: no-op)
    fn emit_event(&self, event: StorageEvent) -> StorageResult<()> {
        // Default implementation: do nothing
        Ok(())
    }
}

/// Helper trait with JSON serialization/deserialization methods
/// 
/// This trait is separated from the main StorageBackend trait
/// to maintain object safety while providing JSON conversion utilities.
pub trait JsonStorageHelper: StorageBackend {
    /// Store a serializable value as JSON
    fn set_json<T: Serialize>(&mut self, key: &str, value: &T) -> StorageResult<()> {
        let json = serde_json::to_string_pretty(value)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.set(key, &json)
    }
    
    /// Get a JSON value and deserialize it
    fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> StorageResult<T> {
        let json = self.get(key)?;
        serde_json::from_str(&json)
            .map_err(|e| StorageError::SerializationError(e.to_string()))
    }
}

// Implement the JsonStorageHelper for all StorageBackend implementors
impl<T: StorageBackend> JsonStorageHelper for T {}

/// Trait for federation-capable storage backends
///
/// This trait extends the basic StorageBackend with methods to support
/// federation and synchronization between ICN nodes.
pub trait FederatedStorageBackend: StorageBackend {
    /// Synchronize with a remote storage backend
    fn synchronize(&mut self, remote: &Box<dyn StorageBackend>) -> StorageResult<()>;
    
    /// Push local changes to a remote storage backend
    fn push(&self, remote: &mut Box<dyn StorageBackend>) -> StorageResult<()>;
    
    /// Pull changes from a remote storage backend
    fn pull(&mut self, remote: &Box<dyn StorageBackend>) -> StorageResult<()>;
    
    /// Resolve conflicts between local and remote storage
    fn resolve_conflicts(&mut self, remote: &Box<dyn StorageBackend>) -> StorageResult<()>;
}

/// In-memory implementation of the StorageBackend trait
///
/// This implementation stores key-value pairs in memory and is primarily
/// used for testing and development. Data is lost when the program exits.
#[derive(Debug, Clone, Default)]
pub struct InMemoryStorage {
    data: HashMap<String, String>,
    transaction_data: Option<HashMap<String, String>>,
    versioned_data: HashMap<String, Vec<(VersionInfo, String)>>,
    audit_log: Vec<StorageEvent>,
    resource_accounts: HashMap<String, ResourceAccount>,
}

impl InMemoryStorage {
    /// Create a new empty in-memory storage
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            transaction_data: None,
            versioned_data: HashMap::new(),
            audit_log: Vec::new(),
            resource_accounts: HashMap::new(),
        }
    }
    
    /// Get a specific resource account
    pub fn get_resource_account(&self, id: &str) -> Option<&ResourceAccount> {
        self.resource_accounts.get(id)
    }
    
    /// Get a mutable reference to a resource account
    pub fn get_resource_account_mut(&mut self, id: &str) -> Option<&mut ResourceAccount> {
        self.resource_accounts.get_mut(id)
    }
    
    /// Create a new resource account
    pub fn create_resource_account(&mut self, id: &str, quota: f64) -> &mut ResourceAccount {
        self.resource_accounts.entry(id.to_string())
            .or_insert_with(|| ResourceAccount::new(id, quota))
    }
    
    /// Get the audit log
    pub fn get_audit_log(&self) -> &[StorageEvent] {
        &self.audit_log
    }
    
    /// Add an entry to the versioned data
    fn add_version(&mut self, key: &str, value: &str, auth: &AuthContext) -> StorageResult<()> {
        let mut versions = self.versioned_data.entry(key.to_string()).or_default();
        let version = versions.len() + 1;
        
        let version_info = VersionInfo {
            version,
            timestamp: now(),
            author: auth.caller.clone(),
            comment: None,
        };
        
        let mut new_versions = versions.clone();
        new_versions.push((version_info, value.to_string()));
        self.versioned_data.insert(key.to_string(), new_versions);
        Ok(())
    }
}

impl StorageBackend for InMemoryStorage {
    fn get(&self, key: &str) -> StorageResult<String> {
        // If in a transaction, check transaction data first
        if let Some(transaction) = &self.transaction_data {
            if let Some(value) = transaction.get(key) {
                return Ok(value.clone());
            }
        }
        
        // Otherwise check main data
        self.data.get(key)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))
    }
    
    fn set(&mut self, key: &str, value: &str) -> StorageResult<()> {
        // If in a transaction, store in transaction data
        if let Some(transaction) = &mut self.transaction_data {
            transaction.insert(key.to_string(), value.to_string());
        } else {
            // Otherwise store in main data
            self.data.insert(key.to_string(), value.to_string());
        }
        Ok(())
    }
    
    fn delete(&mut self, key: &str) -> StorageResult<()> {
        // If in a transaction, mark deletion in transaction
        if let Some(transaction) = &mut self.transaction_data {
            transaction.remove(key);
        } else if self.data.remove(key).is_none() {
            return Err(StorageError::KeyNotFound(key.to_string()));
        }
        Ok(())
    }
    
    fn contains(&self, key: &str) -> bool {
        // If in a transaction, check transaction data first
        if let Some(transaction) = &self.transaction_data {
            if transaction.contains_key(key) {
                return true;
            }
        }
        
        // Otherwise check main data
        self.data.contains_key(key)
    }
    
    fn list_keys(&self, prefix: Option<&str>) -> Vec<String> {
        let mut keys = Vec::new();
        
        // Get keys from base storage first
        for key in self.data.keys() {
            if let Some(prefix_str) = prefix {
                if key.starts_with(prefix_str) {
                    keys.push(key.clone());
                }
            } else {
                keys.push(key.clone());
            }
        }
        
        // Update with transaction data if available
        if let Some(transaction) = &self.transaction_data {
            for key in transaction.keys() {
                if let Some(prefix_str) = prefix {
                    if key.starts_with(prefix_str) && !keys.contains(key) {
                        keys.push(key.clone());
                    }
                } else if !keys.contains(key) {
                    keys.push(key.clone());
                }
            }
        }
        
        keys
    }
    
    fn begin_transaction(&mut self) -> StorageResult<()> {
        if self.transaction_data.is_some() {
            return Err(StorageError::TransactionError("Transaction already in progress".to_string()));
        }
        self.transaction_data = Some(HashMap::new());
        Ok(())
    }
    
    fn commit_transaction(&mut self) -> StorageResult<()> {
        if let Some(transaction) = self.transaction_data.take() {
            // Apply changes to main data
            for (key, value) in transaction {
                self.data.insert(key, value);
            }
            Ok(())
        } else {
            Err(StorageError::TransactionError("No transaction in progress".to_string()))
        }
    }
    
    fn rollback_transaction(&mut self) -> StorageResult<()> {
        if self.transaction_data.is_some() {
            self.transaction_data = None;
            Ok(())
        } else {
            Err(StorageError::TransactionError("No transaction in progress".to_string()))
        }
    }
    
    fn get_with_auth(&self, auth: &AuthContext, key: &str) -> StorageResult<String> {
        // Implement RBAC checks
        if key.starts_with("governance/") {
            // Governance data requires admin or member role
            if !auth.has_role("admin") && !auth.has_role("member") {
                return Err(StorageError::PermissionDenied(format!(
                    "Access to governance data requires admin or member role"
                )));
            }
        }
        
        // Log the access
        let event = StorageEvent::Access {
            key: key.to_string(),
            action: "get".to_string(),
            user: auth.caller.clone(),
            timestamp: auth.timestamp,
        };
        self.emit_event(event)?;
        
        // Call the normal get
        self.get(key)
    }
    
    fn set_with_auth(&mut self, auth: &AuthContext, key: &str, value: &str) -> StorageResult<()> {
        // Implement RBAC checks
        if key.starts_with("governance/") {
            // Governance data requires admin role
            if !auth.has_role("admin") {
                return Err(StorageError::PermissionDenied(format!(
                    "Writing to governance data requires admin role"
                )));
            }
        }
        
        // Log the access
        let event = StorageEvent::Access {
            key: key.to_string(),
            action: "set".to_string(),
            user: auth.caller.clone(),
            timestamp: auth.timestamp,
        };
        self.emit_event(event)?;
        
        // Add a version
        self.add_version(key, value, auth)?;
        
        // Call the normal set
        self.set(key, value)
    }
    
    fn delete_with_auth(&mut self, auth: &AuthContext, key: &str) -> StorageResult<()> {
        // Implement RBAC checks
        if key.starts_with("governance/") {
            // Governance data requires admin role
            if !auth.has_role("admin") {
                return Err(StorageError::PermissionDenied(format!(
                    "Deleting governance data requires admin role"
                )));
            }
        }
        
        // Log the access
        let event = StorageEvent::Access {
            key: key.to_string(),
            action: "delete".to_string(),
            user: auth.caller.clone(),
            timestamp: auth.timestamp,
        };
        self.emit_event(event)?;
        
        // Call the normal delete
        self.delete(key)
    }
    
    fn get_versioned(&self, key: &str, version: usize) -> StorageResult<String> {
        // Look up versions for this key
        let versions = self.versioned_data.get(key)
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))?;
        
        // Find the requested version
        for (info, value) in versions {
            if info.version == version {
                return Ok(value.clone());
            }
        }
        
        // Version not found
        Err(StorageError::VersionNotFound(format!(
            "Version {} not found for key {}",
            version, key
        )))
    }
    
    fn list_versions(&self, key: &str) -> StorageResult<Vec<VersionInfo>> {
        // Look up versions for this key
        let versions = self.versioned_data.get(key)
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))?;
        
        // Extract just the version info
        let infos = versions.iter()
            .map(|(info, _)| info.clone())
            .collect();
        
        Ok(infos)
    }
    
    fn emit_event(&self, event: StorageEvent) -> StorageResult<()> {
        // In a real implementation, this would log to a persistent store
        // or emit events to interested parties
        // For now, just log to the console
        println!("Storage event: {:?}", event);
        Ok(())
    }
}

impl FederatedStorageBackend for InMemoryStorage {
    fn synchronize(&mut self, remote: &Box<dyn StorageBackend>) -> StorageResult<()> {
        // This is just a placeholder implementation for the trait
        // A real implementation would need to handle conflict resolution
        
        // Get all keys from the remote
        let remote_keys = remote.list_keys(None);
        
        // For each remote key, copy the value to local storage
        for key in remote_keys {
            if let Ok(value) = remote.get(&key) {
                self.set(&key, &value)?;
            }
        }
        
        Ok(())
    }
    
    fn push(&self, remote: &mut Box<dyn StorageBackend>) -> StorageResult<()> {
        // Get all local keys
        let local_keys = self.list_keys(None);
        
        // For each local key, copy the value to remote storage
        for key in local_keys {
            if let Ok(value) = self.get(&key) {
                remote.set(&key, &value)?;
            }
        }
        
        Ok(())
    }
    
    fn pull(&mut self, remote: &Box<dyn StorageBackend>) -> StorageResult<()> {
        // Get all remote keys
        let remote_keys = remote.list_keys(None);
        
        // For each remote key, copy the value to local storage
        for key in remote_keys {
            if let Ok(value) = remote.get(&key) {
                self.set(&key, &value)?;
            }
        }
        
        Ok(())
    }
    
    fn resolve_conflicts(&mut self, remote: &Box<dyn StorageBackend>) -> StorageResult<()> {
        // This is just a placeholder implementation
        // A real implementation would need to detect and resolve conflicts
        
        // For simplicity, we'll just synchronize with the remote
        self.synchronize(remote)
    }
}

/// File-based implementation of the StorageBackend trait
///
/// This implementation persists key-value pairs to the filesystem using
/// a simple flat-file structure. Each key is a file path, and the value
/// is stored in the file.
#[derive(Debug)]
pub struct FileStorage {
    /// Base directory for storing files
    base_dir: PathBuf,
    
    /// Optional transaction data
    transaction_data: Option<HashMap<String, Option<String>>>,
    
    /// Audit log path
    audit_log_path: PathBuf,
}

impl FileStorage {
    /// Create a new file-based storage with the given base directory
    pub fn new(base_dir: PathBuf) -> Self {
        // Create directory if it doesn't exist
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir).expect("Failed to create storage directory");
        }
        
        let audit_log_path = base_dir.join("audit_log.json");
        
        Self {
            base_dir,
            transaction_data: None,
            audit_log_path,
        }
    }
    
    /// Convert a key to a file path
    fn key_to_path(&self, key: &str) -> PathBuf {
        // Sanitize the key to ensure it's a valid file path
        let safe_key = key.replace('/', "_").replace('\\', "_");
        self.base_dir.join(safe_key)
    }
}

// FileStorage implementation will be added in the next phase of development
// This is just a placeholder for now until we implement the storage backend
// Basic operations for the VM will be prioritized first 