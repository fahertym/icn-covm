//! Storage Operations Implementation
//!
//! This module implements the StorageOpHandler trait for the VM execution environment.
//! It handles all operations related to persistent storage, including:
//! - Setting up the storage backend
//! - Authentication and authorization
//! - Storage key/value operations
//! - Error handling and mapping to VM errors

use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::Storage;
use crate::vm::errors::VMError;
use crate::vm::MissingKeyBehavior;
use crate::vm::ops::StorageOpHandler;

use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Implementation of storage operations for the VM
#[derive(Debug, Clone)]
pub struct StorageOpImpl<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Storage backend for persistent operations
    storage_backend: Option<S>,

    /// Authentication context for the current execution
    auth_context: Option<AuthContext>,

    /// Storage namespace for current execution
    namespace: String,
}

impl<S> StorageOpImpl<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new storage operations handler
    pub fn new() -> Self {
        Self {
            storage_backend: None,
            auth_context: None,
            namespace: "default".to_string(),
        }
    }

    /// Map storage errors to VM errors
    pub fn map_storage_error(err: StorageError) -> VMError {
        match err {
            StorageError::AuthenticationError { details } => {
                VMError::InvalidSignature {
                    identity_id: "unknown".to_string(),
                    reason: details,
                }
            }
            StorageError::PermissionDenied {
                user_id,
                action,
                key,
            } => VMError::PermissionDenied {
                identity_id: user_id,
                action,
                resource: key,
            },
            StorageError::ResourceNotFound { key, namespace } => {
                VMError::ResourceNotFound {
                    resource: key,
                    namespace,
                }
            }
            StorageError::ResourceAlreadyExists { key, namespace } => {
                VMError::ResourceAlreadyExists {
                    resource: key,
                    namespace,
                }
            }
            StorageError::InsufficientBalance {
                resource,
                account,
                required,
                available,
            } => VMError::InsufficientBalance {
                resource,
                account,
                required,
                available,
            },
            StorageError::InvalidAmount { amount } => {
                VMError::InvalidAmount { amount }
            }
            StorageError::InvalidFormat { details } => {
                VMError::InvalidFormat { reason: details }
            }
            StorageError::StorageEngineError { details } => {
                VMError::StorageError { details }
            }
            StorageError::TransactionError { details } => {
                VMError::TransactionError { details }
            }
            StorageError::NotImplemented { feature } => {
                VMError::NotImplemented { feature }
            }
            StorageError::IdentityNotFound { identity_id } => {
                VMError::IdentityNotFound { identity_id }
            }
            StorageError::InvalidIdentity { reason } => {
                VMError::InvalidIdentity { reason }
            }
            StorageError::VersionNotFound { key, version } => {
                VMError::VersionNotFound { key, version }
            }
            StorageError::ConfigurationError { details } => {
                VMError::ConfigurationError { details }
            }
            StorageError::ValidationError { details } => {
                VMError::ValidationError { details }
            }
            StorageError::Other { details } => VMError::Other { details },
        }
    }

    /// Execute a storage operation with proper error handling
    pub(crate) fn storage_operation<F, T>(
        &mut self,
        operation_name: &str,
        mut f: F,
    ) -> Result<T, VMError>
    where
        F: FnMut(&mut S, Option<&AuthContext>, &str) -> StorageResult<T>,
    {
        match &mut self.storage_backend {
            Some(backend) => {
                let auth_context = self.auth_context.as_ref();
                match f(backend, auth_context, &self.namespace) {
                    Ok(value) => Ok(value),
                    Err(err) => Err(Self::map_storage_error(err)),
                }
            }
            None => Err(VMError::NoStorageBackend),
        }
    }
}

impl<S> StorageOpHandler<S> for StorageOpImpl<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    fn set_storage_backend(&mut self, backend: S) {
        self.storage_backend = Some(backend);
    }

    fn set_auth_context(&mut self, auth: AuthContext) {
        self.auth_context = Some(auth);
    }

    fn set_namespace(&mut self, namespace: &str) {
        self.namespace = namespace.to_string();
    }

    fn get_auth_context(&self) -> Option<&AuthContext> {
        self.auth_context.as_ref()
    }

    fn execute_store_p(&mut self, key: &str, value: f64) -> Result<(), VMError> {
        self.storage_operation("store_p", |storage, auth, namespace| {
            storage.store_float(key, value, auth, namespace)
        })
    }

    fn execute_load_p(
        &mut self,
        key: &str,
        missing_key_behavior: MissingKeyBehavior,
    ) -> Result<f64, VMError> {
        let result = self.storage_operation("load_p", |storage, auth, namespace| {
            storage.load_float(key, auth, namespace)
        });

        match result {
            Ok(value) => Ok(value),
            Err(VMError::ResourceNotFound { .. }) => match missing_key_behavior {
                MissingKeyBehavior::ReturnZero => Ok(0.0),
                MissingKeyBehavior::ReturnNaN => Ok(f64::NAN),
                MissingKeyBehavior::Error => Err(VMError::ResourceNotFound {
                    resource: key.to_string(),
                    namespace: self.namespace.clone(),
                }),
            },
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::InMemoryStorage;
    use crate::storage::traits::Storage;

    #[test]
    fn test_store_and_load() {
        let mut storage_impl = StorageOpImpl::new();
        let backend = InMemoryStorage::new();
        storage_impl.set_storage_backend(backend);

        // Store a value
        storage_impl.execute_store_p("test_key", 42.0).unwrap();

        // Load the value
        let value = storage_impl
            .execute_load_p("test_key", MissingKeyBehavior::Error)
            .unwrap();
        assert_eq!(value, 42.0);
    }

    #[test]
    fn test_missing_key_behavior() {
        let mut storage_impl = StorageOpImpl::new();
        let backend = InMemoryStorage::new();
        storage_impl.set_storage_backend(backend);

        // Test ReturnZero behavior
        let value = storage_impl
            .execute_load_p("nonexistent_key", MissingKeyBehavior::ReturnZero)
            .unwrap();
        assert_eq!(value, 0.0);

        // Test ReturnNaN behavior
        let value = storage_impl
            .execute_load_p("nonexistent_key", MissingKeyBehavior::ReturnNaN)
            .unwrap();
        assert!(value.is_nan());

        // Test Error behavior
        let result = storage_impl.execute_load_p("nonexistent_key", MissingKeyBehavior::Error);
        assert!(matches!(result, Err(VMError::ResourceNotFound { .. })));
    }

    #[test]
    fn test_no_storage_backend() {
        let mut storage_impl = StorageOpImpl::<InMemoryStorage>::new();
        
        // Try to store without a backend
        let result = storage_impl.execute_store_p("test_key", 42.0);
        assert!(matches!(result, Err(VMError::NoStorageBackend)));
    }

    #[test]
    fn test_different_namespaces() {
        let mut storage_impl = StorageOpImpl::new();
        let backend = InMemoryStorage::new();
        storage_impl.set_storage_backend(backend);

        // Store a value in default namespace
        storage_impl.execute_store_p("test_key", 42.0).unwrap();
        
        // Change namespace
        storage_impl.set_namespace("other_namespace");
        
        // Verify key doesn't exist in new namespace
        let result = storage_impl.execute_load_p("test_key", MissingKeyBehavior::Error);
        assert!(matches!(result, Err(VMError::ResourceNotFound { .. })));

        // Store value in new namespace
        storage_impl.execute_store_p("test_key", 99.0).unwrap();
        
        // Load value from new namespace
        let value = storage_impl
            .execute_load_p("test_key", MissingKeyBehavior::Error)
            .unwrap();
        assert_eq!(value, 99.0);
    }
} 