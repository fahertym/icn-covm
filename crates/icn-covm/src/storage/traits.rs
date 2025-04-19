use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
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

    /// Stores data as JSON in storage with authentication context from current user
    fn set_json_authed<T: Serialize>(
        &mut self,
        auth_context: &AuthContext,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()> {
        self.set_json(Some(auth_context), namespace, key, value)
    }

    /// Gets data as JSON from storage with authentication context from current user
    fn get_json_authed<T: DeserializeOwned>(
        &self,
        auth_context: &AuthContext,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T> {
        self.get_json(Some(auth_context), namespace, key)
    }

    /// Check if a key exists with authentication context
    fn contains_authed(
        &self,
        auth_context: &AuthContext,
        namespace: &str,
        key: &str,
    ) -> StorageResult<bool> {
        self.contains(Some(auth_context), namespace, key)
    }

    /// List keys in a namespace with authentication context
    fn list_keys_authed(
        &self,
        auth_context: &AuthContext,
        namespace: &str,
        prefix: Option<&str>,
    ) -> StorageResult<Vec<String>> {
        self.list_keys(Some(auth_context), namespace, prefix)
    }

    /// Delete a key with authentication context
    fn delete_authed(
        &mut self,
        auth_context: &AuthContext,
        namespace: &str,
        key: &str,
    ) -> StorageResult<()> {
        self.delete(Some(auth_context), namespace, key)
    }

    /// Store versioning-aware JSON data with built-in conflict detection
    fn set_json_versioned<T: Serialize + DeserializeOwned>(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: &T,
        expected_version: Option<u64>,
    ) -> StorageResult<u64> {
        // Check if we need to verify the version
        if let Some(expected) = expected_version {
            // Get the current version info
            if self.contains(auth, namespace, key)? {
                let (_, version_info) = self.get_versioned(auth, namespace, key)?;

                // Check for version mismatch
                if version_info.version != expected {
                    return Err(StorageError::VersionConflict {
                        current: version_info.version,
                        expected,
                        resource: key.to_string(),
                    });
                }
            }
        }

        // Store the value
        self.set_json(auth, namespace, key, value)?;

        // Get the new version number
        let (_, version_info) = self.get_versioned(auth, namespace, key)?;
        Ok(version_info.version)
    }
}

// Blanket impl for all types implementing StorageBackend
impl<S: StorageBackend> StorageExtensions for S {
    fn get_identity(&self, identity_id: &str) -> StorageResult<crate::identity::Identity> {
        let key = format!("identities/{}", identity_id);
        let bytes = self.get(None, "identity", &key)?;
        serde_json::from_slice(&bytes).map_err(|e| {
            crate::storage::errors::StorageError::SerializationError {
                data_type: "Identity".to_string(),
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
                data_type: std::any::type_name::<T>().to_string(),
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
                data_type: std::any::type_name::<T>().to_string(),
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
        match self.get_version(auth, namespace, key, version) {
            Ok((bytes, _)) => serde_json::from_slice(&bytes).map(Some).map_err(|e| {
                crate::storage::errors::StorageError::SerializationError {
                    data_type: std::any::type_name::<T>().to_string(),
                    details: e.to_string(),
                }
            }),
            Err(crate::storage::errors::StorageError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// EconomicOperations provides operations for managing resources and accounts
pub trait EconomicOperations: StorageBackend {
    /// Create a new economic resource
    fn create_resource(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        resource: &str,
    ) -> StorageResult<()> {
        // Default implementation creates a resource metadata entry
        let key = format!("resources/{}/metadata", resource);
        let metadata = format!(
            "{{\"id\": \"{}\", \"namespace\": \"{}\"}}",
            resource, namespace
        );
        self.set(auth, namespace, &key, metadata.as_bytes().to_vec())?;
        Ok(())
    }

    /// Mint new units of a resource for an account
    fn mint(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        resource: &str,
        account: &str,
        amount: u64,
        reason: &str,
    ) -> StorageResult<((), Option<StorageEvent>)> {
        // Check if resource exists
        let resource_key = format!("resources/{}/metadata", resource);
        if !self.contains(auth, namespace, &resource_key)? {
            return Err(StorageError::ResourceNotFound(resource.to_string()));
        }

        // Get current balance
        let balance_key = format!("resources/{}/accounts/{}", resource, account);
        let current_balance = if self.contains(auth, namespace, &balance_key)? {
            match std::str::from_utf8(&self.get(auth, namespace, &balance_key)?) {
                Ok(s) => s.parse::<u64>().unwrap_or(0),
                Err(_) => 0,
            }
        } else {
            0
        };

        // Update balance
        let new_balance = current_balance + amount;
        self.set(
            auth,
            namespace,
            &balance_key,
            new_balance.to_string().as_bytes().to_vec(),
        )?;

        // Create event
        let event = StorageEvent {
            user_id: auth
                .map(|a| a.user_id_string())
                .unwrap_or_else(|| "system".to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            namespace: namespace.to_string(),
            key: balance_key,
            event_type: "mint".to_string(),
            details: format!(
                "Minted {} of {} for {}: {}",
                amount, resource, account, reason
            ),
        };

        Ok(((), Some(event)))
    }

    /// Transfer resource units between accounts
    fn transfer(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        resource: &str,
        from: &str,
        to: &str,
        amount: u64,
        reason: &str,
    ) -> StorageResult<((), Option<StorageEvent>)> {
        // Check if resource exists
        let resource_key = format!("resources/{}/metadata", resource);
        if !self.contains(auth, namespace, &resource_key)? {
            return Err(StorageError::ResourceNotFound(resource.to_string()));
        }

        // Get from balance
        let from_key = format!("resources/{}/accounts/{}", resource, from);
        let from_balance = if self.contains(auth, namespace, &from_key)? {
            match std::str::from_utf8(&self.get(auth, namespace, &from_key)?) {
                Ok(s) => s.parse::<u64>().unwrap_or(0),
                Err(_) => 0,
            }
        } else {
            0
        };

        // Check if sufficient balance
        if from_balance < amount {
            return Err(StorageError::InsufficientBalance(format!(
                "Account {} has insufficient balance for resource {}",
                from, resource
            )));
        }

        // Get to balance
        let to_key = format!("resources/{}/accounts/{}", resource, to);
        let to_balance = if self.contains(auth, namespace, &to_key)? {
            match std::str::from_utf8(&self.get(auth, namespace, &to_key)?) {
                Ok(s) => s.parse::<u64>().unwrap_or(0),
                Err(_) => 0,
            }
        } else {
            0
        };

        // Update balances
        let new_from_balance = from_balance - amount;
        let new_to_balance = to_balance + amount;

        self.set(
            auth,
            namespace,
            &from_key,
            new_from_balance.to_string().as_bytes().to_vec(),
        )?;
        self.set(
            auth,
            namespace,
            &to_key,
            new_to_balance.to_string().as_bytes().to_vec(),
        )?;

        // Create event
        let event = StorageEvent {
            user_id: auth
                .map(|a| a.user_id_string())
                .unwrap_or_else(|| "system".to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            namespace: namespace.to_string(),
            key: format!("{}->{}", from_key, to_key),
            event_type: "transfer".to_string(),
            details: format!(
                "Transferred {} of {} from {} to {}: {}",
                amount, resource, from, to, reason
            ),
        };

        Ok(((), Some(event)))
    }

    /// Burn resource units from an account
    fn burn(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        resource: &str,
        account: &str,
        amount: u64,
        reason: &str,
    ) -> StorageResult<((), Option<StorageEvent>)> {
        // Check if resource exists
        let resource_key = format!("resources/{}/metadata", resource);
        if !self.contains(auth, namespace, &resource_key)? {
            return Err(StorageError::ResourceNotFound(resource.to_string()));
        }

        // Get current balance
        let balance_key = format!("resources/{}/accounts/{}", resource, account);
        let current_balance = if self.contains(auth, namespace, &balance_key)? {
            match std::str::from_utf8(&self.get(auth, namespace, &balance_key)?) {
                Ok(s) => s.parse::<u64>().unwrap_or(0),
                Err(_) => 0,
            }
        } else {
            0
        };

        // Check if sufficient balance
        if current_balance < amount {
            return Err(StorageError::InsufficientBalance(format!(
                "Account {} has insufficient balance for resource {}",
                account, resource
            )));
        }

        // Update balance
        let new_balance = current_balance - amount;
        self.set(
            auth,
            namespace,
            &balance_key,
            new_balance.to_string().as_bytes().to_vec(),
        )?;

        // Create event
        let event = StorageEvent {
            user_id: auth
                .map(|a| a.user_id_string())
                .unwrap_or_else(|| "system".to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            namespace: namespace.to_string(),
            key: balance_key,
            event_type: "burn".to_string(),
            details: format!(
                "Burned {} of {} from {}: {}",
                amount, resource, account, reason
            ),
        };

        Ok(((), Some(event)))
    }

    /// Get the balance of a resource for an account
    fn get_balance(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        resource: &str,
        account: &str,
    ) -> StorageResult<(u64, Option<StorageEvent>)> {
        // Check if resource exists
        let resource_key = format!("resources/{}/metadata", resource);
        if !self.contains(auth, namespace, &resource_key)? {
            return Err(StorageError::ResourceNotFound(resource.to_string()));
        }

        // Get balance
        let balance_key = format!("resources/{}/accounts/{}", resource, account);
        let balance = if self.contains(auth, namespace, &balance_key)? {
            match std::str::from_utf8(&self.get(auth, namespace, &balance_key)?) {
                Ok(s) => s.parse::<u64>().unwrap_or(0),
                Err(_) => 0,
            }
        } else {
            0
        };

        // Create event
        let event = StorageEvent {
            user_id: auth
                .map(|a| a.user_id_string())
                .unwrap_or_else(|| "system".to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            namespace: namespace.to_string(),
            key: balance_key,
            event_type: "get_balance".to_string(),
            details: format!(
                "Retrieved balance of {} for {}: {}",
                resource, account, balance
            ),
        };

        Ok((balance, Some(event)))
    }

    /// Get reputation for an identity
    fn get_reputation(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        identity_id: &str,
    ) -> StorageResult<(u64, Option<StorageEvent>)> {
        // Get reputation
        let rep_key = format!("identities/{}/reputation", identity_id);
        let reputation = if self.contains(auth, namespace, &rep_key)? {
            match std::str::from_utf8(&self.get(auth, namespace, &rep_key)?) {
                Ok(s) => s.parse::<u64>().unwrap_or(0),
                Err(_) => 0,
            }
        } else {
            0
        };

        // No event for reading reputation
        Ok((reputation, None))
    }

    /// Set reputation for an identity
    fn set_reputation(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        identity_id: &str,
        value: u64,
    ) -> StorageResult<((), Option<StorageEvent>)> {
        // Set reputation
        let rep_key = format!("identities/{}/reputation", identity_id);
        self.set(
            auth,
            namespace,
            &rep_key,
            value.to_string().as_bytes().to_vec(),
        )?;

        // Create event
        let event = StorageEvent {
            user_id: auth
                .map(|a| a.user_id_string())
                .unwrap_or_else(|| "system".to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            namespace: namespace.to_string(),
            key: rep_key,
            event_type: "set_reputation".to_string(),
            details: format!("Set reputation for {} to {}", identity_id, value),
        };

        Ok(((), Some(event)))
    }

    /// Store custom data
    fn store(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: Vec<u8>,
    ) -> StorageResult<((), Option<StorageEvent>)> {
        // Set the value
        self.set(auth, namespace, key, value)?;

        // Create event
        let event = StorageEvent {
            user_id: auth
                .map(|a| a.user_id_string())
                .unwrap_or_else(|| "system".to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            namespace: namespace.to_string(),
            key: key.to_string(),
            event_type: "store".to_string(),
            details: format!("Stored data for key {}", key),
        };

        Ok(((), Some(event)))
    }

    /// Load custom data
    fn load(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<(Vec<u8>, Option<StorageEvent>)> {
        // Get the data
        let data = self.get(auth, namespace, key)?;

        // Create event
        let event = StorageEvent {
            user_id: auth
                .map(|a| a.user_id_string())
                .unwrap_or_else(|| "system".to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            namespace: namespace.to_string(),
            key: key.to_string(),
            event_type: "load".to_string(),
            details: format!("Loaded data for key {}", key),
        };

        Ok((data, Some(event)))
    }
}

// Automatically implement EconomicOperations for all StorageBackend implementors
impl<T: StorageBackend> EconomicOperations for T {}

/// Define a standard Storage type that includes all trait bounds
pub trait Storage: StorageBackend + EconomicOperations + Clone + Send + Sync {}

/// Blanket implementation for the Storage supertrait.
impl<T: StorageBackend + EconomicOperations + Clone + Send + Sync> Storage for T {}
