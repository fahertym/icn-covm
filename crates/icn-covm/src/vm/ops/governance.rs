//! Governance Operations Implementation
//!
//! This module implements the GovernanceOpHandler trait for the VM execution environment.
//! It handles operations related to resource management, economic actions, and
//! governance operations including:
//! - Resource creation and management
//! - Minting, transferring, and burning resource units
//! - Querying resource balances
//! - Authorization and validation of governance actions

use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::Storage;
use crate::typed::TypedValue;
use crate::vm::errors::VMError;
use crate::vm::ops::GovernanceOpHandler;
use crate::vm::ops::storage::StorageOpImpl;

use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Implementation of governance operations for the VM
#[derive(Debug, Clone)]
pub struct GovernanceOpImpl<S>
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

impl<S> GovernanceOpImpl<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new governance operations handler
    pub fn new() -> Self {
        Self {
            storage_backend: None,
            auth_context: None,
            namespace: "default".to_string(),
        }
    }

    /// Extract a numeric value from a TypedValue, with validation
    fn extract_numeric_amount(&self, amount: &TypedValue) -> Result<f64, VMError> {
        match amount.as_number() {
            Ok(num) if num >= 0.0 => Ok(num),
            Ok(num) => Err(VMError::InvalidAmount { amount: num }),
            Err(_) => Err(VMError::TypeMismatch {
                expected: "Number".to_string(),
                found: amount.type_name().to_string(),
                operation: "resource operation".to_string(),
            }),
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

impl<S> GovernanceOpHandler<S> for GovernanceOpImpl<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    fn execute_create_resource(&mut self, resource: &str) -> Result<(), VMError> {
        self.storage_operation("create_resource", |storage, auth, namespace| {
            storage.create_resource(resource, auth, namespace)
        })
    }

    fn execute_mint(
        &mut self,
        resource: &str,
        account: &str,
        amount: &TypedValue,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        // Extract and validate numeric amount
        let numeric_amount = self.extract_numeric_amount(amount)?;

        // Execute the mint operation
        self.storage_operation("mint", |storage, auth, namespace| {
            storage.mint(
                resource,
                account,
                numeric_amount,
                reason.as_deref().unwrap_or("VM mint operation"),
                auth,
                namespace,
            )
        })
    }

    fn execute_transfer(
        &mut self,
        resource: &str,
        from: &str,
        to: &str,
        amount: &TypedValue,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        // Extract and validate numeric amount
        let numeric_amount = self.extract_numeric_amount(amount)?;

        // Execute the transfer operation
        self.storage_operation("transfer", |storage, auth, namespace| {
            storage.transfer(
                resource,
                from,
                to,
                numeric_amount,
                reason.as_deref().unwrap_or("VM transfer operation"),
                auth,
                namespace,
            )
        })
    }

    fn execute_burn(
        &mut self,
        resource: &str,
        account: &str,
        amount: &TypedValue,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        // Extract and validate numeric amount
        let numeric_amount = self.extract_numeric_amount(amount)?;

        // Execute the burn operation
        self.storage_operation("burn", |storage, auth, namespace| {
            storage.burn(
                resource,
                account,
                numeric_amount,
                reason.as_deref().unwrap_or("VM burn operation"),
                auth,
                namespace,
            )
        })
    }

    fn execute_balance(&mut self, resource: &str, account: &str) -> Result<TypedValue, VMError> {
        let balance = self.storage_operation("balance", |storage, auth, namespace| {
            storage.balance(resource, account, auth, namespace)
        })?;
        
        Ok(TypedValue::Number(balance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::InMemoryStorage;

    #[test]
    fn test_create_resource() {
        let mut gov_impl = GovernanceOpImpl::new();
        let backend = InMemoryStorage::new();
        gov_impl.storage_backend = Some(backend);

        // Create a resource
        gov_impl.execute_create_resource("test_resource").unwrap();

        // Creating the same resource should fail
        let result = gov_impl.execute_create_resource("test_resource");
        assert!(matches!(result, Err(VMError::ResourceAlreadyExists { .. })));
    }

    #[test]
    fn test_mint_and_balance() {
        let mut gov_impl = GovernanceOpImpl::new();
        let backend = InMemoryStorage::new();
        gov_impl.storage_backend = Some(backend);

        // Create a resource
        gov_impl.execute_create_resource("test_resource").unwrap();

        // Mint some units
        gov_impl
            .execute_mint("test_resource", "user1", &TypedValue::Number(100.0), &None)
            .unwrap();

        // Check the balance
        let balance = gov_impl.execute_balance("test_resource", "user1").unwrap();
        assert_eq!(balance, TypedValue::Number(100.0));
    }

    #[test]
    fn test_invalid_mint_amount() {
        let mut gov_impl = GovernanceOpImpl::new();
        let backend = InMemoryStorage::new();
        gov_impl.storage_backend = Some(backend);

        // Create a resource
        gov_impl.execute_create_resource("test_resource").unwrap();

        // Try to mint negative amount
        let result = gov_impl.execute_mint(
            "test_resource", 
            "user1", 
            &TypedValue::Number(-100.0), 
            &None
        );
        assert!(matches!(result, Err(VMError::InvalidAmount { .. })));

        // Try to mint with non-numeric value
        let result = gov_impl.execute_mint(
            "test_resource", 
            "user1", 
            &TypedValue::String("not a number".to_string()), 
            &None
        );
        assert!(matches!(result, Err(VMError::TypeMismatch { .. })));
    }

    #[test]
    fn test_transfer() {
        let mut gov_impl = GovernanceOpImpl::new();
        let backend = InMemoryStorage::new();
        gov_impl.storage_backend = Some(backend);

        // Create a resource
        gov_impl.execute_create_resource("test_resource").unwrap();

        // Mint some units
        gov_impl
            .execute_mint("test_resource", "user1", &TypedValue::Number(100.0), &None)
            .unwrap();

        // Transfer some units
        gov_impl
            .execute_transfer(
                "test_resource",
                "user1",
                "user2",
                &TypedValue::Number(50.0),
                &None,
            )
            .unwrap();

        // Check balances
        let balance1 = gov_impl.execute_balance("test_resource", "user1").unwrap();
        let balance2 = gov_impl.execute_balance("test_resource", "user2").unwrap();
        
        assert_eq!(balance1, TypedValue::Number(50.0));
        assert_eq!(balance2, TypedValue::Number(50.0));
    }

    #[test]
    fn test_insufficient_balance() {
        let mut gov_impl = GovernanceOpImpl::new();
        let backend = InMemoryStorage::new();
        gov_impl.storage_backend = Some(backend);

        // Create a resource
        gov_impl.execute_create_resource("test_resource").unwrap();

        // Mint some units
        gov_impl
            .execute_mint("test_resource", "user1", &TypedValue::Number(100.0), &None)
            .unwrap();

        // Try to transfer more than available
        let result = gov_impl.execute_transfer(
            "test_resource",
            "user1",
            "user2",
            &TypedValue::Number(150.0),
            &None,
        );
        
        assert!(matches!(result, Err(VMError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_burn() {
        let mut gov_impl = GovernanceOpImpl::new();
        let backend = InMemoryStorage::new();
        gov_impl.storage_backend = Some(backend);

        // Create a resource
        gov_impl.execute_create_resource("test_resource").unwrap();

        // Mint some units
        gov_impl
            .execute_mint("test_resource", "user1", &TypedValue::Number(100.0), &None)
            .unwrap();

        // Burn some units
        gov_impl
            .execute_burn("test_resource", "user1", &TypedValue::Number(30.0), &None)
            .unwrap();

        // Check balance
        let balance = gov_impl.execute_balance("test_resource", "user1").unwrap();
        assert_eq!(balance, TypedValue::Number(70.0));

        // Try to burn more than available
        let result = gov_impl.execute_burn(
            "test_resource",
            "user1",
            &TypedValue::Number(100.0),
            &None,
        );
        
        assert!(matches!(result, Err(VMError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_nonexistent_resource() {
        let mut gov_impl = GovernanceOpImpl::new();
        let backend = InMemoryStorage::new();
        gov_impl.storage_backend = Some(backend);

        // Try operations on nonexistent resource
        let result = gov_impl.execute_mint(
            "nonexistent_resource",
            "user1",
            &TypedValue::Number(100.0),
            &None,
        );
        assert!(matches!(result, Err(VMError::ResourceNotFound { .. })));

        let result = gov_impl.execute_balance("nonexistent_resource", "user1");
        assert!(matches!(result, Err(VMError::ResourceNotFound { .. })));
    }
} 