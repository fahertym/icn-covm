//! VM Operation execution logic
//!
//! This module provides the execution logic for VM operations.
//!
//! The execution module handles:
//! - Storage backend interactions
//! - Authentication and authorization
//! - Operation execution semantics
//! - Event generation and tracking
//! - Transaction management
//!
//! Separating execution logic provides:
//! - Clear boundaries for operation implementations
//! - Isolation of storage interaction complexity
//! - Focused testing of execution behaviors
//! - Potential for alternative execution strategies
//! - Easier implementation of new operation types
//!
//! The module defines an `ExecutorOps` trait that encapsulates operation execution,
//! enabling alternative implementations for different execution models.

use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::Storage;
use crate::vm::errors::VMError;
use crate::vm::types::VMEvent;
use crate::vm::MissingKeyBehavior;
use crate::typed::TypedValue;
use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Defines operations for VM execution logic
pub trait ExecutorOps<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Set the storage backend
    fn set_storage_backend(&mut self, backend: S);

    /// Set the authentication context
    fn set_auth_context(&mut self, auth: AuthContext);

    /// Set the namespace
    fn set_namespace(&mut self, namespace: &str);

    /// Get the authentication context
    fn get_auth_context(&self) -> Option<&AuthContext>;

    /// Execute a resource creation operation
    fn execute_create_resource(&mut self, resource: &str) -> Result<(), VMError>;

    /// Execute a minting operation
    fn execute_mint(
        &mut self,
        resource: &str,
        account: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError>;

    /// Execute a transfer operation
    fn execute_transfer(
        &mut self,
        resource: &str,
        from: &str,
        to: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError>;

    /// Execute a burn operation
    fn execute_burn(
        &mut self,
        resource: &str,
        account: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError>;

    /// Execute a balance query operation
    fn execute_balance(&mut self, resource: &str, account: &str) -> Result<f64, VMError>;

    /// Execute increment reputation for an identity
    fn execute_increment_reputation(
        &mut self,
        identity_id: &str,
        amount: Option<f64>,
    ) -> Result<(), VMError>;

    /// Execute a storage operation with the given key/value
    fn execute_store_p(&mut self, key: &str, value: f64) -> Result<(), VMError>;

    /// Load a value from storage
    fn execute_load_p(
        &mut self,
        key: &str,
        missing_key_behavior: MissingKeyBehavior,
    ) -> Result<f64, VMError>;

    /// Fork the VM for transaction support
    fn fork(&mut self) -> Result<Self, VMError>
    where
        Self: Sized;

    /// Commit a transaction from a forked VM
    fn commit_fork_transaction(&mut self) -> Result<(), VMError>;

    /// Rollback a transaction from a forked VM
    fn rollback_fork_transaction(&mut self) -> Result<(), VMError>;

    /// Emit a message to the output
    fn emit(&mut self, message: &str);

    /// Emit an event with the given category and message
    fn emit_event(&mut self, category: &str, message: &str);

    /// Get the current output buffer
    fn get_output(&self) -> &str;

    /// Get the events as a vector
    fn get_events(&self) -> &[VMEvent];

    /// Clear the output buffer
    fn clear_output(&mut self);

    /// Execute arithmetic operations
    fn execute_arithmetic(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError>;

    /// Execute comparison operations
    fn execute_comparison(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError>;

    /// Execute logical operations
    fn execute_logical(&self, a: &TypedValue, op: &str) -> Result<TypedValue, VMError>;

    /// Execute binary logical operations
    fn execute_binary_logical(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError>;
}

/// Provides execution logic for the virtual machine operations
#[derive(Debug)]
pub struct VMExecution<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Storage backend for persistent operations
    pub(crate) storage_backend: Option<S>,

    /// Authentication context for the current execution
    pub(crate) auth_context: Option<AuthContext>,

    /// Storage namespace for current execution
    pub(crate) namespace: String,

    /// Output buffer
    pub(crate) output: String,

    /// Event log
    pub(crate) events: Vec<VMEvent>,

    /// Transaction state tracking
    pub(crate) transaction_active: bool,
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
                    Err(err) => Err(match err {
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
                        } => VMError::StorageError {
                            details: format!(
                                "Permission denied for user '{}' during {}: operation '{}' on '{}'",
                                user_id, operation_name, action, key
                            )
                        },
                        StorageError::NotFound { key } => VMError::StorageError {
                            details: format!(
                                "Key '{}' not found during {}",
                                key, operation_name
                            )
                        },
                        _ => VMError::StorageError {
                            details: format!(
                                "Error during {}: {:?}",
                                operation_name, err
                            )
                        },
                    }),
                }
            }
            None => Err(VMError::StorageUnavailable),
        }
    }

    /// Convert a storage event to a VM event
    fn storage_event_to_vm_event(
        &self,
        storage_event: &crate::storage::events::StorageEvent,
        category: &str,
    ) -> VMEvent {
        VMEvent {
            category: category.to_string(),
            message: format!("{}: {}", storage_event.event_type, storage_event.details),
            timestamp: storage_event.timestamp,
        }
    }
}

impl<S> ExecutorOps<S> for VMExecution<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Set the storage backend
    fn set_storage_backend(&mut self, backend: S) {
        self.storage_backend = Some(backend);
    }

    /// Set the authentication context
    fn set_auth_context(&mut self, auth: AuthContext) {
        self.auth_context = Some(auth);
    }

    /// Set the namespace
    fn set_namespace(&mut self, namespace: &str) {
        self.namespace = namespace.to_string();
    }

    /// Get the authentication context
    fn get_auth_context(&self) -> Option<&AuthContext> {
        self.auth_context.as_ref()
    }

    /// Execute a minting operation
    fn execute_mint(
        &mut self,
        resource: &str,
        account: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        let reason_str = reason
            .clone()
            .unwrap_or_else(|| "No reason provided".to_string());

        self.storage_operation("mint", |backend, auth, namespace| {
            backend
                .mint(
                    auth,
                    namespace,
                    resource,
                    account,
                    amount as u64,
                    &reason_str,
                )
                .map(|(_, event_opt)| {
                    // Log any event generated
                    if let Some(storage_event) = event_opt {
                        // Create VM event
                        let vm_event = VMEvent {
                            category: "economic".to_string(),
                            message: format!("mint: {}", storage_event.details),
                            timestamp: storage_event.timestamp,
                        };
                        // Return VMEvent for logging outside this closure
                        Some(vm_event)
                    } else {
                        None
                    }
                })
        })
        .map(|event_opt| {
            // Log the event if one was generated
            if let Some(event) = event_opt {
                self.events.push(event);
            }
        })
    }

    /// Execute a transfer operation
    fn execute_transfer(
        &mut self,
        resource: &str,
        from: &str,
        to: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        let reason_str = reason
            .clone()
            .unwrap_or_else(|| "No reason provided".to_string());

        self.storage_operation("transfer", |backend, auth, namespace| {
            backend
                .transfer(
                    auth,
                    namespace,
                    resource,
                    from,
                    to,
                    amount as u64,
                    &reason_str,
                )
                .map(|(_, event_opt)| {
                    // Log any event generated
                    if let Some(storage_event) = event_opt {
                        // Create VM event
                        let vm_event = VMEvent {
                            category: "economic".to_string(),
                            message: format!("transfer: {}", storage_event.details),
                            timestamp: storage_event.timestamp,
                        };
                        // Return VMEvent for logging outside this closure
                        Some(vm_event)
                    } else {
                        None
                    }
                })
        })
        .map(|event_opt| {
            // Log the event if one was generated
            if let Some(event) = event_opt {
                self.events.push(event);
            }
        })
    }

    /// Execute a burn operation
    fn execute_burn(
        &mut self,
        resource: &str,
        account: &str,
        amount: f64,
        reason: &Option<String>,
    ) -> Result<(), VMError> {
        let reason_str = reason
            .clone()
            .unwrap_or_else(|| "No reason provided".to_string());

        self.storage_operation("burn", |backend, auth, namespace| {
            backend
                .burn(
                    auth,
                    namespace,
                    resource,
                    account,
                    amount as u64,
                    &reason_str,
                )
                .map(|(_, event_opt)| {
                    // Log any event generated
                    if let Some(storage_event) = event_opt {
                        // Create VM event
                        let vm_event = VMEvent {
                            category: "economic".to_string(),
                            message: format!("burn: {}", storage_event.details),
                            timestamp: storage_event.timestamp,
                        };
                        // Return VMEvent for logging outside this closure
                        Some(vm_event)
                    } else {
                        None
                    }
                })
        })
        .map(|event_opt| {
            // Log the event if one was generated
            if let Some(event) = event_opt {
                self.events.push(event);
            }
        })
    }

    /// Execute a balance query operation
    fn execute_balance(&mut self, resource: &str, account: &str) -> Result<f64, VMError> {
        self.storage_operation("get_balance", |backend, auth, namespace| {
            backend
                .get_balance(auth, namespace, resource, account)
                .map(|(balance, event_opt)| {
                    // Log any event generated
                    if let Some(storage_event) = event_opt {
                        // Create VM event
                        let vm_event = VMEvent {
                            category: "economic".to_string(),
                            message: format!("balance: {}", storage_event.details),
                            timestamp: storage_event.timestamp,
                        };
                        // Push the event to the VM event log
                        (balance as f64, Some(vm_event))
                    } else {
                        (balance as f64, None)
                    }
                })
        })
        .map(|(balance, event_opt)| {
            // Log the event if one was generated
            if let Some(event) = event_opt {
                self.events.push(event);
            }
            // Return the balance
            balance
        })
    }

    /// Execute a resource creation operation
    fn execute_create_resource(&mut self, resource: &str) -> Result<(), VMError> {
        // Create the resource and emit event
        self.storage_operation("create_resource", |backend, auth, namespace| {
            backend.create_resource(auth, namespace, resource)
        })?;

        // Create and log an event for resource creation
        let event = VMEvent {
            category: "economic".to_string(),
            message: format!("Resource created: {}", resource),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        self.events.push(event);

        Ok(())
    }

    /// Execute increment reputation for an identity
    fn execute_increment_reputation(
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
            let current = self
                .storage_operation("get_reputation", |backend, auth, namespace| {
                    backend
                        .get_reputation(auth, namespace, identity_id)
                        .map(|(rep, event_opt)| {
                            // Log any event generated
                            if let Some(storage_event) = event_opt {
                                // Create VM event
                                let vm_event = VMEvent {
                                    category: "reputation".to_string(),
                                    message: format!("get_reputation: {}", storage_event.details),
                                    timestamp: storage_event.timestamp,
                                };
                                // Return the reputation value and event
                                (rep, Some(vm_event))
                            } else {
                                (rep, None)
                            }
                        })
                })
                .map(|(rep, event_opt)| {
                    // Log the event if one was generated
                    if let Some(event) = event_opt {
                        self.events.push(event);
                    }
                    // Return the reputation value
                    rep
                })?;

            // Update reputation
            self.storage_operation("set_reputation", |backend, auth, namespace| {
                backend
                    .set_reputation(auth, namespace, identity_id, current + amount_val)
                    .map(|(_, event_opt)| {
                        // Log any event generated
                        if let Some(storage_event) = event_opt {
                            // Create VM event
                            let vm_event = VMEvent {
                                category: "reputation".to_string(),
                                message: format!("set_reputation: {}", storage_event.details),
                                timestamp: storage_event.timestamp,
                            };
                            // Return the event
                            Some(vm_event)
                        } else {
                            None
                        }
                    })
            })
            .map(|event_opt| {
                // Log the event if one was generated
                if let Some(event) = event_opt {
                    self.events.push(event);
                }
            })?;
        }

        Ok(())
    }

    /// Execute a storage operation with the given key/value
    fn execute_store_p(&mut self, key: &str, value: f64) -> Result<(), VMError> {
        self.storage_operation("store_p", |backend, auth, namespace| {
            backend
                .store(auth, namespace, key, value.to_string().as_bytes().to_vec())
                .map(|(_, event_opt)| {
                    // Log any event generated
                    if let Some(storage_event) = event_opt {
                        // Create VM event
                        let vm_event = VMEvent {
                            category: "storage".to_string(),
                            message: format!("store: {}", storage_event.details),
                            timestamp: storage_event.timestamp,
                        };
                        // Return the event
                        Some(vm_event)
                    } else {
                        None
                    }
                })
        })
        .map(|event_opt| {
            // Log the event if one was generated
            if let Some(event) = event_opt {
                self.events.push(event);
            }
        })
    }

    /// Load a value from storage
    fn execute_load_p(
        &mut self,
        key: &str,
        missing_key_behavior: MissingKeyBehavior,
    ) -> Result<f64, VMError> {
        let bytes = match self.storage_operation("load_p", |backend, auth, namespace| {
            backend.load(auth, namespace, key).map(|(data, event_opt)| {
                // Log any event generated
                if let Some(storage_event) = event_opt {
                    // Create VM event
                    let vm_event = VMEvent {
                        category: "storage".to_string(),
                        message: format!("load: {}", storage_event.details),
                        timestamp: storage_event.timestamp,
                    };
                    // Return the data and event
                    (data, Some(vm_event))
                } else {
                    (data, None)
                }
            })
        }) {
            Ok(result) => {
                // Process any events that were returned
                if let Some(event) = result.1 {
                    self.events.push(event);
                }
                Ok(result.0) // Extract just the data part from the tuple
            }
            Err(VMError::StorageError { details: ref err_msg }) if err_msg.contains("not found") => {
                match missing_key_behavior {
                    MissingKeyBehavior::Default => Ok(vec![48, 46, 48]) /* "0.0" in ASCII */,
                    MissingKeyBehavior::Error => Err(VMError::StorageError {
                        details: format!(
                            "Key '{}' not found during load_p",
                            key
                        )
                    }),
                }
            }
            Err(e) => Err(e),
        }?;

        let value_str = String::from_utf8(bytes).map_err(|_| {
            VMError::Deserialization(format!("Failed to parse value for key '{}'", key))
        })?;

        value_str.parse::<f64>().map_err(|_| {
            VMError::Deserialization(format!("Failed to parse value as f64 for key '{}'", key))
        })
    }

    /// Fork the VM for transaction support
    fn fork(&mut self) -> Result<Self, VMError> {
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
                        VMError::StorageError {
                            details: format!("Failed to begin transaction: {:?}", e)
                        }
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
    fn commit_fork_transaction(&mut self) -> Result<(), VMError> {
        if !self.transaction_active {
            return Err(VMError::StorageError {
                details: "No active transaction to commit".to_string(),
            });
        }

        if let Some(backend) = &mut self.storage_backend {
            backend.commit_transaction().map_err(|e| {
                VMError::StorageError {
                    details: format!("Failed to commit transaction: {:?}", e)
                }
            })?;
        }

        self.transaction_active = false;
        Ok(())
    }

    /// Rollback a transaction from a forked VM
    fn rollback_fork_transaction(&mut self) -> Result<(), VMError> {
        if !self.transaction_active {
            return Err(VMError::StorageError {
                details: "No active transaction to rollback".to_string(),
            });
        }

        if let Some(backend) = &mut self.storage_backend {
            backend.rollback_transaction().map_err(|e| {
                VMError::StorageError {
                    details: format!("Failed to rollback transaction: {:?}", e)
                }
            })?;
        }

        self.transaction_active = false;
        Ok(())
    }

    /// Emit a message to the output
    fn emit(&mut self, message: &str) {
        self.output.push_str(message);
        self.output.push('\n');
    }

    /// Emit an event with the given category and message
    fn emit_event(&mut self, category: &str, message: &str) {
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
    fn get_output(&self) -> &str {
        &self.output
    }

    /// Get the events as a vector
    fn get_events(&self) -> &[VMEvent] {
        &self.events
    }

    /// Clear the output buffer
    fn clear_output(&mut self) {
        self.output.clear();
    }

    /// Execute arithmetic operations
    fn execute_arithmetic(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError> {
        // Extract number values or return a type error
        let a_num = a.as_number().map_err(|_| VMError::TypeError {
            expected: "number".to_string(),
            found: a.type_name().to_string(),
            op_name: "arithmetic".to_string(),
        })?;
        
        let b_num = b.as_number().map_err(|_| VMError::TypeError {
            expected: "number".to_string(),
            found: b.type_name().to_string(),
            op_name: "arithmetic".to_string(),
        })?;
        
        let result = match op {
            "add" => a_num + b_num,
            "sub" => a_num - b_num,
            "mul" => a_num * b_num,
            "div" => {
                if b_num == 0.0 {
                    return Err(VMError::DivisionByZero);
                } else {
                    a_num / b_num
                }
            }
            "mod" => {
                if b_num == 0.0 {
                    return Err(VMError::DivisionByZero);
                } else {
                    a_num % b_num
                }
            }
            _ => {
                return Err(VMError::NotImplemented(format!(
                    "Unknown arithmetic operation: {}",
                    op
                )))
            }
        };
        
        Ok(TypedValue::Number(result))
    }

    /// Execute comparison operations
    fn execute_comparison(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError> {
        // Extract number values or return a type error
        let a_num = a.as_number().map_err(|_| VMError::TypeError {
            expected: "number".to_string(),
            found: a.type_name().to_string(),
            op_name: "comparison".to_string(),
        })?;
        
        let b_num = b.as_number().map_err(|_| VMError::TypeError {
            expected: "number".to_string(),
            found: b.type_name().to_string(),
            op_name: "comparison".to_string(),
        })?;
        
        let result = match op {
            "eq" => (a_num - b_num).abs() < f64::EPSILON,
            "lt" => a_num < b_num,
            "gt" => a_num > b_num,
            _ => {
                return Err(VMError::NotImplemented(format!(
                    "Unknown comparison operation: {}",
                    op
                )))
            }
        };
        
        Ok(TypedValue::Boolean(result))
    }

    /// Execute logical operations
    fn execute_logical(&self, a: &TypedValue, op: &str) -> Result<TypedValue, VMError> {
        // For NOT operation
        let result = match op {
            "not" => a.is_falsey(),
            _ => {
                return Err(VMError::NotImplemented(format!(
                    "Unknown logical operation: {}",
                    op
                )))
            }
        };
        
        // Convert boolean result to TypedValue
        Ok(TypedValue::Boolean(result))
    }

    /// Execute binary logical operations
    fn execute_binary_logical(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError> {
        // For binary operations (AND, OR)
        let a_truthy = !a.is_falsey();
        let b_truthy = !b.is_falsey();

        let result = match op {
            "and" => a_truthy && b_truthy,
            "or" => a_truthy || b_truthy,
            _ => {
                return Err(VMError::NotImplemented(format!(
                    "Unknown binary logical operation: {}",
                    op
                )))
            }
        };

        // Convert boolean result to TypedValue
        Ok(TypedValue::Boolean(result))
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

        assert_eq!(
            exec.execute_arithmetic(
                &TypedValue::Number(5.0), 
                &TypedValue::Number(3.0), 
                "add"
            ).unwrap(), 
            TypedValue::Number(8.0)
        );
        
        assert_eq!(
            exec.execute_arithmetic(
                &TypedValue::Number(5.0), 
                &TypedValue::Number(3.0), 
                "sub"
            ).unwrap(), 
            TypedValue::Number(2.0)
        );
        
        assert_eq!(
            exec.execute_arithmetic(
                &TypedValue::Number(5.0), 
                &TypedValue::Number(3.0), 
                "mul"
            ).unwrap(), 
            TypedValue::Number(15.0)
        );
        
        assert_eq!(
            exec.execute_arithmetic(
                &TypedValue::Number(6.0), 
                &TypedValue::Number(3.0), 
                "div"
            ).unwrap(), 
            TypedValue::Number(2.0)
        );
        
        assert_eq!(
            exec.execute_arithmetic(
                &TypedValue::Number(7.0), 
                &TypedValue::Number(3.0), 
                "mod"
            ).unwrap(), 
            TypedValue::Number(1.0)
        );

        // Test division by zero
        assert!(matches!(
            exec.execute_arithmetic(
                &TypedValue::Number(5.0), 
                &TypedValue::Number(0.0), 
                "div"
            ),
            Err(VMError::DivisionByZero)
        ));
        
        // Test type error
        assert!(matches!(
            exec.execute_arithmetic(
                &TypedValue::String("not a number".to_string()), 
                &TypedValue::Number(5.0), 
                "add"
            ),
            Err(VMError::TypeError { .. })
        ));
    }

    #[test]
    fn test_comparison_operations() {
        let exec = VMExecution::<InMemoryStorage>::new();

        // Equal
        assert_eq!(
            exec.execute_comparison(
                &TypedValue::Number(5.0), 
                &TypedValue::Number(5.0), 
                "eq"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_comparison(
                &TypedValue::Number(5.0), 
                &TypedValue::Number(3.0), 
                "eq"
            ).unwrap(), 
            TypedValue::Boolean(false)
        );

        // Less than
        assert_eq!(
            exec.execute_comparison(
                &TypedValue::Number(3.0), 
                &TypedValue::Number(5.0), 
                "lt"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_comparison(
                &TypedValue::Number(5.0), 
                &TypedValue::Number(3.0), 
                "lt"
            ).unwrap(), 
            TypedValue::Boolean(false)
        );

        // Greater than
        assert_eq!(
            exec.execute_comparison(
                &TypedValue::Number(5.0), 
                &TypedValue::Number(3.0), 
                "gt"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_comparison(
                &TypedValue::Number(3.0), 
                &TypedValue::Number(5.0), 
                "gt"
            ).unwrap(), 
            TypedValue::Boolean(false)
        );
    }

    #[test]
    fn test_logical_operations() {
        let exec = VMExecution::<InMemoryStorage>::new();

        // NOT with various types
        assert_eq!(
            exec.execute_logical(&TypedValue::Number(0.0), "not").unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_logical(&TypedValue::Number(1.0), "not").unwrap(), 
            TypedValue::Boolean(false)
        );
        
        assert_eq!(
            exec.execute_logical(&TypedValue::Boolean(false), "not").unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_logical(&TypedValue::String("".to_string()), "not").unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_logical(&TypedValue::String("hello".to_string()), "not").unwrap(), 
            TypedValue::Boolean(false)
        );

        // AND
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(0.0), 
                &TypedValue::Number(0.0), 
                "and"
            ).unwrap(), 
            TypedValue::Boolean(false)
        );
        
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(1.0), 
                &TypedValue::Number(0.0), 
                "and"
            ).unwrap(), 
            TypedValue::Boolean(false)
        );
        
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(0.0), 
                &TypedValue::Number(1.0), 
                "and"
            ).unwrap(), 
            TypedValue::Boolean(false)
        );
        
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(1.0), 
                &TypedValue::Number(1.0), 
                "and"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );

        // OR
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(0.0), 
                &TypedValue::Number(0.0), 
                "or"
            ).unwrap(), 
            TypedValue::Boolean(false)
        );
        
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(1.0), 
                &TypedValue::Number(0.0), 
                "or"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(0.0), 
                &TypedValue::Number(1.0), 
                "or"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(1.0), 
                &TypedValue::Number(1.0), 
                "or"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );
        
        // Test with mixed types
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::Number(1.0), 
                &TypedValue::Boolean(true), 
                "and"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );
        
        assert_eq!(
            exec.execute_binary_logical(
                &TypedValue::String("hello".to_string()), 
                &TypedValue::Number(0.0), 
                "or"
            ).unwrap(), 
            TypedValue::Boolean(true)
        );
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
