//! Identity Operations Implementation
//!
//! This module implements the IdentityOpHandler trait for the VM execution environment.
//! It handles operations related to identity management, including:
//! - Reputation management
//! - Identity verification
//! - Membership checking
//! - Delegation management

use crate::identity::Identity;
use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::Storage;
use crate::vm::errors::VMError;
use crate::vm::ops::IdentityOpHandler;
use crate::vm::ops::storage::StorageOpImpl;

use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Implementation of identity operations for the VM
#[derive(Debug, Clone)]
pub struct IdentityOpImpl<S>
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

impl<S> IdentityOpImpl<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new identity operations handler
    pub fn new() -> Self {
        Self {
            storage_backend: None,
            auth_context: None,
            namespace: "default".to_string(),
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
                    Err(err) => Err(StorageOpImpl::<S>::map_storage_error(err)),
                }
            }
            None => Err(VMError::NoStorageBackend),
        }
    }
}

impl<S> IdentityOpHandler<S> for IdentityOpImpl<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    fn execute_increment_reputation(
        &mut self,
        identity_id: &str,
        amount: Option<f64>,
    ) -> Result<(), VMError> {
        let increment_amount = amount.unwrap_or(1.0);
        
        // Validate amount
        if increment_amount <= 0.0 {
            return Err(VMError::InvalidAmount { amount: increment_amount });
        }

        // Verify identity exists
        self.storage_operation("get_identity", |storage, auth, namespace| {
            storage.get_identity(identity_id, auth, namespace)
        })?;

        // Get current reputation
        let reputation_key = format!("reputation:{}", identity_id);
        let current_reputation = match self.storage_operation("load_float", |storage, auth, namespace| {
            storage.load_float(&reputation_key, auth, namespace)
        }) {
            Ok(value) => value,
            Err(VMError::ResourceNotFound { .. }) => 0.0,
            Err(err) => return Err(err),
        };

        // Increment reputation
        let new_reputation = current_reputation + increment_amount;
        self.storage_operation("store_float", |storage, auth, namespace| {
            storage.store_float(&reputation_key, new_reputation, auth, namespace)
        })
    }

    fn execute_verify_identity(
        &mut self,
        identity_id: &str,
        message: &str,
        signature: &str,
    ) -> Result<bool, VMError> {
        // Get the identity
        let identity = self.storage_operation("get_identity", |storage, auth, namespace| {
            storage.get_identity(identity_id, auth, namespace)
        })?;

        // Verify the signature
        match identity.verify_signature(message, signature) {
            Ok(valid) => Ok(valid),
            Err(err) => Err(VMError::InvalidSignature {
                identity_id: identity_id.to_string(),
                reason: err.to_string(),
            }),
        }
    }

    fn execute_check_membership(
        &mut self,
        identity_id: &str,
        namespace: &str,
    ) -> Result<bool, VMError> {
        self.storage_operation("check_membership", |storage, auth, _| {
            storage.check_membership(identity_id, namespace, auth)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;
    use crate::storage::memory::InMemoryStorage;

    #[test]
    fn test_increment_reputation() {
        let mut id_impl = IdentityOpImpl::new();
        let mut backend = InMemoryStorage::new();
        
        // Create an identity
        let identity = Identity::new("test_user".to_string());
        backend.store_identity(&identity, None, "default").unwrap();
        
        id_impl.storage_backend = Some(backend);

        // Increment reputation
        id_impl.execute_increment_reputation("test_user", None).unwrap();
        
        // Check reputation (by directly accessing storage)
        if let Some(backend) = &mut id_impl.storage_backend {
            let reputation = backend
                .load_float("reputation:test_user", None, "default")
                .unwrap();
            assert_eq!(reputation, 1.0);
        }

        // Increment again with specific amount
        id_impl.execute_increment_reputation("test_user", Some(2.5)).unwrap();
        
        // Check again
        if let Some(backend) = &mut id_impl.storage_backend {
            let reputation = backend
                .load_float("reputation:test_user", None, "default")
                .unwrap();
            assert_eq!(reputation, 3.5);
        }
    }

    #[test]
    fn test_invalid_reputation_increment() {
        let mut id_impl = IdentityOpImpl::new();
        let mut backend = InMemoryStorage::new();
        
        // Create an identity
        let identity = Identity::new("test_user".to_string());
        backend.store_identity(&identity, None, "default").unwrap();
        
        id_impl.storage_backend = Some(backend);

        // Try to increment with negative amount
        let result = id_impl.execute_increment_reputation("test_user", Some(-1.0));
        assert!(matches!(result, Err(VMError::InvalidAmount { .. })));
    }

    #[test]
    fn test_nonexistent_identity() {
        let mut id_impl = IdentityOpImpl::new();
        let backend = InMemoryStorage::new();
        id_impl.storage_backend = Some(backend);

        // Try to increment reputation for nonexistent identity
        let result = id_impl.execute_increment_reputation("nonexistent", None);
        assert!(matches!(result, Err(VMError::IdentityNotFound { .. })));
    }

    // Test signature verification and membership checking would require more complex setup
    // with actual cryptographic operations, so we'll omit them for now
} 