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
use crate::typed::TypedValue;
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

    fn execute_store_p(&mut self, key: &str, value: &TypedValue) -> Result<(), VMError> {
        match value {
            TypedValue::Number(num) => {
                // For numeric values, use the existing float storage
                self.storage_operation("store_p", |storage, auth, namespace| {
                    storage.store_float(key, *num, auth, namespace)
                })
            }
            _ => {
                // For non-numeric values, serialize to JSON
                let json_str = serde_json::to_string(value)
                    .map_err(|e| VMError::SerializationError { details: e.to_string() })?;
                
                self.storage_operation("store_p", |storage, auth, namespace| {
                    storage.store_string(key, &json_str, auth, namespace)
                })
            }
        }
    }

    fn execute_load_p(
        &mut self,
        key: &str,
        missing_key_behavior: MissingKeyBehavior,
    ) -> Result<TypedValue, VMError> {
        // First try to load as a float (for backward compatibility)
        let float_result = self.storage_operation("load_p", |storage, auth, namespace| {
            storage.load_float(key, auth, namespace)
        });

        match float_result {
            Ok(value) => Ok(TypedValue::Number(value)),
            Err(VMError::ResourceNotFound { .. }) => {
                // If not found as float, try as string (which might be JSON)
                let string_result = self.storage_operation("load_p", |storage, auth, namespace| {
                    storage.load_string(key, auth, namespace)
                });

                match string_result {
                    Ok(json_str) => {
                        // Try to parse as TypedValue
                        match serde_json::from_str::<TypedValue>(&json_str) {
                            Ok(typed_value) => Ok(typed_value),
                            Err(_) => {
                                // If not valid JSON, treat as string value
                                Ok(TypedValue::String(json_str))
                            }
                        }
                    }
                    Err(VMError::ResourceNotFound { .. }) => match missing_key_behavior {
                        MissingKeyBehavior::ReturnZero => Ok(TypedValue::Number(0.0)),
                        MissingKeyBehavior::ReturnNaN => Ok(TypedValue::Number(f64::NAN)),
                        MissingKeyBehavior::Error => Err(VMError::ResourceNotFound {
                            resource: key.to_string(),
                            namespace: self.namespace.clone(),
                        }),
                    },
                    Err(err) => Err(err),
                }
            }
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
        let mut storage_op = StorageOpImpl::new();
        storage_op.set_storage_backend(InMemoryStorage::new());

        // Store and load a number
        let num_value = TypedValue::Number(42.0);
        storage_op.execute_store_p("test_number", &num_value).unwrap();
        let loaded_num = storage_op.execute_load_p("test_number", MissingKeyBehavior::Error).unwrap();
        assert_eq!(loaded_num, num_value);

        // Store and load a string
        let str_value = TypedValue::String("Hello, world!".to_string());
        storage_op.execute_store_p("test_string", &str_value).unwrap();
        let loaded_str = storage_op.execute_load_p("test_string", MissingKeyBehavior::Error).unwrap();
        assert_eq!(loaded_str, str_value);

        // Store and load a boolean
        let bool_value = TypedValue::Boolean(true);
        storage_op.execute_store_p("test_bool", &bool_value).unwrap();
        let loaded_bool = storage_op.execute_load_p("test_bool", MissingKeyBehavior::Error).unwrap();
        assert_eq!(loaded_bool, bool_value);

        // Store and load null
        let null_value = TypedValue::Null;
        storage_op.execute_store_p("test_null", &null_value).unwrap();
        let loaded_null = storage_op.execute_load_p("test_null", MissingKeyBehavior::Error).unwrap();
        assert_eq!(loaded_null, null_value);
    }

    #[test]
    fn test_missing_key_behavior() {
        let mut storage_op = StorageOpImpl::new();
        storage_op.set_storage_backend(InMemoryStorage::new());

        // Test ReturnZero behavior
        let result = storage_op
            .execute_load_p("nonexistent", MissingKeyBehavior::ReturnZero)
            .unwrap();
        assert_eq!(result, TypedValue::Number(0.0));

        // Test ReturnNaN behavior
        let result = storage_op
            .execute_load_p("nonexistent", MissingKeyBehavior::ReturnNaN)
            .unwrap();
        if let TypedValue::Number(num) = result {
            assert!(num.is_nan());
        } else {
            panic!("Expected Number type");
        }

        // Test Error behavior
        let result = storage_op.execute_load_p("nonexistent", MissingKeyBehavior::Error);
        assert!(matches!(result, Err(VMError::ResourceNotFound { .. })));
    }

    #[test]
    fn test_no_storage_backend() {
        let mut storage_op = StorageOpImpl::<InMemoryStorage>::new();

        let result = storage_op.execute_store_p("test", &TypedValue::Number(42.0));
        assert!(matches!(result, Err(VMError::NoStorageBackend)));

        let result = storage_op.execute_load_p("test", MissingKeyBehavior::Error);
        assert!(matches!(result, Err(VMError::NoStorageBackend)));
    }

    #[test]
    fn test_different_namespaces() {
        let mut storage_op = StorageOpImpl::new();
        storage_op.set_storage_backend(InMemoryStorage::new());

        // Store in namespace1
        storage_op.set_namespace("namespace1");
        storage_op.execute_store_p("test", &TypedValue::Number(42.0)).unwrap();

        // Should not be found in namespace2
        storage_op.set_namespace("namespace2");
        let result = storage_op.execute_load_p("test", MissingKeyBehavior::Error);
        assert!(matches!(result, Err(VMError::ResourceNotFound { .. })));

        // Should be found in namespace1
        storage_op.set_namespace("namespace1");
        let result = storage_op.execute_load_p("test", MissingKeyBehavior::Error).unwrap();
        assert_eq!(result, TypedValue::Number(42.0));
    }
} 