//! VM Operation execution logic
//! 
//! This module provides the execution logic for VM operations.

use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::Storage;
use crate::vm::errors::VMError;
use crate::vm::types::{LoopControl, Op, VMEvent};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Provides execution logic for the virtual machine operations
#[derive(Debug)]
pub struct VMExecution<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Storage backend for persistent operations
    storage_backend: Option<S>,

    /// Authentication context for the current execution
    auth_context: Option<AuthContext>,

    /// Storage namespace for current execution
    namespace: String,

    /// Output buffer
    output: String,

    /// Event log
    events: Vec<VMEvent>,

    /// Transaction state tracking
    transaction_active: bool,
}

impl<S> VMExecution<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new execution environment
    pub fn new() -> Self {
        Self {
            storage_backend: None,
            auth_context: None,
            namespace: "default".to_string(),
            output: String::new(),
            events: Vec::new(),
            transaction_active: false,
        }
    }

    /// Set the storage backend
    pub fn set_storage_backend(&mut self, backend: S) {
        self.storage_backend = Some(backend);
    }

    /// Set the authentication context
    pub fn set_auth_context(&mut self, auth: AuthContext) {
        self.auth_context = Some(auth);
    }

    /// Set the namespace
    pub fn set_namespace(&mut self, namespace: &str) {
        self.namespace = namespace.to_string();
    }

    /// Get the authentication context
    pub fn get_auth_context(&self) -> Option<&AuthContext> {
        self.auth_context.as_ref()
    }

    /// Execute a storage operation with proper error handling
    pub fn storage_operation<F, T>(&mut self, operation_name: &str, mut f: F) -> Result<T, VMError>
    where
        F: FnMut(&mut S, Option<&AuthContext>, &str) -> StorageResult<(T, Option<VMEvent>)>,
    {
        let result = match &mut self.storage_backend {
            Some(backend) => {
                let auth_context = self.auth_context.as_ref();
                match f(backend, auth_context, &self.namespace) {
                    Ok((value, maybe_event)) => {
                        // If the operation generated an event, log it
                        if let Some(event) = maybe_event {
                            self.events.push(event);
                        }
                        Ok(value)
                    }
                    Err(err) => Err(match err {
                        StorageError::AuthError(msg) => VMError::InvalidSignature {
                            identity_id: "unknown".to_string(),
                            reason: msg,
                        },
                        StorageError::PermissionDenied(msg) => VMError::StorageError(format!(
                            "Permission denied during {}: {}",
                            operation_name, msg
                        )),
                        StorageError::NotFound(key) => VMError::StorageError(format!(
                            "Key '{}' not found during {}",
                            key, operation_name
                        )),
                        _ => VMError::StorageError(format!(
                            "Error during {}: {:?}",
                            operation_name, err
                        )),
                    }),
                }
            }
            None => Err(VMError::StorageUnavailable),
        };

        result
    }

    /// Execute a minting operation
    pub fn execute_mint(
        &mut self,
        resource: &str,
        account: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        let reason_str = reason.clone().unwrap_or_else(|| "No reason provided".to_string());
        
        self.storage_operation("mint", |backend, auth, namespace| {
            backend.mint(
                auth,
                namespace,
                resource,
                account,
                amount as u64,
                &reason_str,
            )
        })
    }

    /// Execute a transfer operation
    pub fn execute_transfer(
        &mut self,
        resource: &str,
        from: &str,
        to: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        let reason_str = reason.clone().unwrap_or_else(|| "No reason provided".to_string());
        
        self.storage_operation("transfer", |backend, auth, namespace| {
            backend.transfer(auth, namespace, resource, from, to, amount as u64, &reason_str)
        })
    }

    /// Execute a burn operation
    pub fn execute_burn(
        &mut self,
        resource: &str,
        account: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        let reason_str = reason.clone().unwrap_or_else(|| "No reason provided".to_string());
        
        self.storage_operation("burn", |backend, auth, namespace| {
            backend.burn(auth, namespace, resource, account, amount as u64, &reason_str)
        })
    }

    /// Execute a balance query operation
    pub fn execute_balance(&mut self, resource: &str, account: &str) -> Result<f64, VMError> {
        let result = self.storage_operation("get_balance", |backend, auth, namespace| {
            backend.get_balance(auth, namespace, resource, account)
        })?;
        
        Ok(result as f64)
    }

    /// Execute a resource creation operation
    pub fn execute_create_resource(&mut self, resource: &str) -> Result<(), VMError> {
        self.storage_operation("create_resource", |backend, auth, namespace| {
            backend.create_resource(auth, namespace, resource)
        })
    }

    /// Execute increment reputation for an identity
    pub fn execute_increment_reputation(
        &mut self,
        identity_id: &str,
        amount: Option<f64>,
    ) -> Result<(), VMError> {
        let amount_val = amount.unwrap_or(1.0) as u64;

        // Prepare the payload
        let payload = format!(
            r#"{{"identity_id": "{}", "amount": {}}}"#,
            identity_id, amount_val
        );

        // Emit an event for the reputation change
        self.emit_event("reputation", &payload);

        // If we have a storage backend, persist the reputation
        if self.storage_backend.is_some() {
            // Get current reputation
            let current = self.storage_operation("get_reputation", |backend, auth, namespace| {
                backend.get_reputation(auth, namespace, identity_id)
            })?;

            // Update reputation
            let new_value = current + amount_val;
            self.storage_operation("set_reputation", |backend, auth, namespace| {
                backend.set_reputation(auth, namespace, identity_id, new_value)
            })?;
        }

        Ok(())
    }

    /// Execute a conditional block and return the result
    pub fn execute_conditional_block(&mut self, ops: &[Op]) -> Result<f64, VMError> {
        // This is a placeholder - the actual execution will happen in the VM
        // We'll just indicate success or failure for now
        Ok(1.0)
    }

    /// Execute a storage operation with the given key/value
    pub fn execute_store_p(&mut self, key: &str, value: f64) -> Result<(), VMError> {
        self.storage_operation("store_p", |backend, auth, namespace| {
            backend.store(auth, namespace, key, value.to_string().as_bytes())
        })
    }

    /// Load a value from storage
    pub fn execute_load_p(&mut self, key: &str) -> Result<f64, VMError> {
        let bytes = self.storage_operation("load_p", |backend, auth, namespace| {
            backend.load(auth, namespace, key)
        })?;

        let value_str = String::from_utf8(bytes).map_err(|_| {
            VMError::Deserialization(format!("Failed to parse value for key '{}'", key))
        })?;

        value_str.parse::<f64>().map_err(|_| {
            VMError::Deserialization(format!("Failed to parse value as f64 for key '{}'", key))
        })
    }

    /// Fork the VM for transaction support
    pub fn fork(&mut self) -> Result<Self, VMError> {
        // Clone the storage backend if available
        let storage_fork = match &self.storage_backend {
            Some(backend) => {
                let forked_backend = backend.clone();
                // Start a transaction
                let mut forked = Self {
                    storage_backend: Some(forked_backend),
                    auth_context: self.auth_context.clone(),
                    namespace: self.namespace.clone(),
                    output: self.output.clone(),
                    events: Vec::new(), // Start with empty events, we'll merge later if committed
                    transaction_active: true,
                };
                
                if let Some(backend) = &mut forked.storage_backend {
                    backend.begin_transaction().map_err(|e| {
                        VMError::StorageError(format!("Failed to begin transaction: {:?}", e))
                    })?;
                }
                
                Some(forked)
            }
            None => None,
        };

        match storage_fork {
            Some(forked) => Ok(forked),
            None => Err(VMError::StorageUnavailable),
        }
    }

    /// Commit a transaction from a forked VM
    pub fn commit_fork_transaction(&mut self) -> Result<(), VMError> {
        if !self.transaction_active {
            return Err(VMError::StorageError(
                "No active transaction to commit".to_string(),
            ));
        }

        if let Some(backend) = &mut self.storage_backend {
            backend.commit_transaction().map_err(|e| {
                VMError::StorageError(format!("Failed to commit transaction: {:?}", e))
            })?;
        }

        self.transaction_active = false;
        Ok(())
    }

    /// Rollback a transaction from a forked VM
    pub fn rollback_fork_transaction(&mut self) -> Result<(), VMError> {
        if !self.transaction_active {
            return Err(VMError::StorageError(
                "No active transaction to rollback".to_string(),
            ));
        }

        if let Some(backend) = &mut self.storage_backend {
            backend.rollback_transaction().map_err(|e| {
                VMError::StorageError(format!("Failed to rollback transaction: {:?}", e))
            })?;
        }

        self.transaction_active = false;
        Ok(())
    }

    /// Emit a message to the output
    pub fn emit(&mut self, message: &str) {
        self.output.push_str(message);
        self.output.push('\n');
    }

    /// Emit an event with the given category and message
    pub fn emit_event(&mut self, category: &str, message: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let event = VMEvent {
            category: category.to_string(),
            message: message.to_string(),
            timestamp: now,
        };

        self.events.push(event);
    }

    /// Get the current output buffer
    pub fn get_output(&self) -> &str {
        &self.output
    }

    /// Get the events as a vector
    pub fn get_events(&self) -> &[VMEvent] {
        &self.events
    }

    /// Clear the output buffer
    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    /// Execute arithmetic operations
    pub fn execute_arithmetic(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError> {
        match op {
            "add" => Ok(a + b),
            "sub" => Ok(a - b),
            "mul" => Ok(a * b),
            "div" => {
                if b == 0.0 {
                    Err(VMError::DivisionByZero)
                } else {
                    Ok(a / b)
                }
            }
            "mod" => {
                if b == 0.0 {
                    Err(VMError::DivisionByZero)
                } else {
                    Ok(a % b)
                }
            }
            _ => Err(VMError::NotImplemented(format!("Unknown arithmetic operation: {}", op))),
        }
    }

    /// Execute comparison operations
    pub fn execute_comparison(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError> {
        // In our VM, 0.0 is falsey and any non-zero value is truthy
        let result = match op {
            "eq" => (a - b).abs() < f64::EPSILON,
            "lt" => a < b,
            "gt" => a > b,
            _ => return Err(VMError::NotImplemented(format!("Unknown comparison operation: {}", op))),
        };
        
        // Convert boolean to f64 (0.0 for false, 1.0 for true)
        Ok(if result { 1.0 } else { 0.0 })
    }

    /// Execute logical operations
    pub fn execute_logical(&self, a: f64, op: &str) -> Result<f64, VMError> {
        // For NOT operation
        let result = match op {
            "not" => a == 0.0, // NOT truthy is falsey, NOT falsey is truthy
            _ => return Err(VMError::NotImplemented(format!("Unknown logical operation: {}", op))),
        };
        
        // Convert boolean to f64 (0.0 for false, 1.0 for true)
        Ok(if result { 1.0 } else { 0.0 })
    }

    /// Execute binary logical operations
    pub fn execute_binary_logical(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError> {
        // For binary operations (AND, OR)
        let a_truthy = a != 0.0;
        let b_truthy = b != 0.0;
        
        let result = match op {
            "and" => a_truthy && b_truthy,
            "or" => a_truthy || b_truthy,
            _ => return Err(VMError::NotImplemented(format!("Unknown binary logical operation: {}", op))),
        };
        
        // Convert boolean to f64 (0.0 for false, 1.0 for true)
        Ok(if result { 1.0 } else { 0.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::implementations::in_memory::InMemoryStorage;

    impl Default for InMemoryStorage {
        fn default() -> Self {
            Self::new()
        }
    }

    #[test]
    fn test_arithmetic_operations() {
        let exec = VMExecution::<InMemoryStorage>::new();
        
        assert_eq!(exec.execute_arithmetic(5.0, 3.0, "add").unwrap(), 8.0);
        assert_eq!(exec.execute_arithmetic(5.0, 3.0, "sub").unwrap(), 2.0);
        assert_eq!(exec.execute_arithmetic(5.0, 3.0, "mul").unwrap(), 15.0);
        assert_eq!(exec.execute_arithmetic(6.0, 3.0, "div").unwrap(), 2.0);
        assert_eq!(exec.execute_arithmetic(7.0, 3.0, "mod").unwrap(), 1.0);
        
        // Test division by zero
        assert!(matches!(
            exec.execute_arithmetic(5.0, 0.0, "div"),
            Err(VMError::DivisionByZero)
        ));
    }

    #[test]
    fn test_comparison_operations() {
        let exec = VMExecution::<InMemoryStorage>::new();
        
        // Equal
        assert_eq!(exec.execute_comparison(5.0, 5.0, "eq").unwrap(), 1.0);
        assert_eq!(exec.execute_comparison(5.0, 3.0, "eq").unwrap(), 0.0);
        
        // Less than
        assert_eq!(exec.execute_comparison(3.0, 5.0, "lt").unwrap(), 1.0);
        assert_eq!(exec.execute_comparison(5.0, 3.0, "lt").unwrap(), 0.0);
        
        // Greater than
        assert_eq!(exec.execute_comparison(5.0, 3.0, "gt").unwrap(), 1.0);
        assert_eq!(exec.execute_comparison(3.0, 5.0, "gt").unwrap(), 0.0);
    }

    #[test]
    fn test_logical_operations() {
        let exec = VMExecution::<InMemoryStorage>::new();
        
        // NOT
        assert_eq!(exec.execute_logical(0.0, "not").unwrap(), 1.0);
        assert_eq!(exec.execute_logical(1.0, "not").unwrap(), 0.0);
        
        // AND
        assert_eq!(exec.execute_binary_logical(0.0, 0.0, "and").unwrap(), 0.0);
        assert_eq!(exec.execute_binary_logical(1.0, 0.0, "and").unwrap(), 0.0);
        assert_eq!(exec.execute_binary_logical(0.0, 1.0, "and").unwrap(), 0.0);
        assert_eq!(exec.execute_binary_logical(1.0, 1.0, "and").unwrap(), 1.0);
        
        // OR
        assert_eq!(exec.execute_binary_logical(0.0, 0.0, "or").unwrap(), 0.0);
        assert_eq!(exec.execute_binary_logical(1.0, 0.0, "or").unwrap(), 1.0);
        assert_eq!(exec.execute_binary_logical(0.0, 1.0, "or").unwrap(), 1.0);
        assert_eq!(exec.execute_binary_logical(1.0, 1.0, "or").unwrap(), 1.0);
    }

    #[test]
    fn test_emit_event() {
        let mut exec = VMExecution::<InMemoryStorage>::new();
        
        exec.emit_event("test", "Test message");
        
        let events = exec.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, "test");
        assert_eq!(events[0].message, "Test message");
    }
} 