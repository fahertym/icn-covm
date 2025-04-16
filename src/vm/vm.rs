//! Main Virtual Machine implementation
//!
//! This module brings together stack, memory, and execution components
//! to implement the main VM functionality.
//!
//! The VM struct is the central coordinator that:
//! - Integrates the stack, memory, and execution subsystems
//! - Implements operation execution logic
//! - Manages control flow
//! - Provides the primary API for VM users
//!
//! This design approach:
//! - Delegates specialized functionality to appropriate subsystems
//! - Maintains clean separation of concerns
//! - Allows for targeted testing of different VM aspects
//! - Provides a solid foundation for extending VM capabilities
//! - Facilitates both AST interpretation and bytecode execution

use crate::events::Event;
use crate::identity::Identity;
use crate::storage::auth::AuthContext;
use crate::storage::errors::StorageResult;
use crate::storage::traits::Storage;
use crate::vm::errors::VMError;
use crate::vm::execution::{ExecutorOps, VMExecution};
use crate::vm::memory::{MemoryScope, VMMemory};
use crate::vm::stack::{StackOps, VMStack};
use crate::vm::types::{CallFrame, LoopControl, Op, VMEvent};

use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::{Send, Sync};

/// The Virtual Machine for cooperative value networks
///
/// This struct coordinates the stack, memory, and execution components
/// to implement the full VM functionality.
#[derive(Debug)]
pub struct VM<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Stack operations
    pub stack: VMStack,

    /// Memory and scope management
    pub memory: VMMemory,

    /// Execution logic
    pub executor: VMExecution<S>,
}

impl<S> VM<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new VM with default settings
    pub fn new() -> Self {
        Self {
            stack: VMStack::new(),
            memory: VMMemory::new(),
            executor: VMExecution::new(),
        }
    }

    /// Create a new VM with a storage backend
    pub fn with_storage_backend(backend: S) -> Self {
        let mut vm = Self::new();
        vm.set_storage_backend(backend);
        vm
    }

    /// Set the storage backend
    pub fn set_storage_backend(&mut self, backend: S) {
        self.executor.set_storage_backend(backend);
    }

    /// Set the authentication context
    pub fn set_auth_context(&mut self, auth: AuthContext) {
        self.executor.set_auth_context(auth);
    }

    /// Set the namespace
    pub fn set_namespace(&mut self, namespace: &str) {
        self.executor.set_namespace(namespace);
    }

    /// Get the authentication context
    pub fn get_auth_context(&self) -> Option<&AuthContext> {
        self.executor.get_auth_context()
    }

    /// Get the storage backend
    pub fn get_storage_backend(&self) -> Option<&S> {
        self.executor.storage_backend.as_ref()
    }

    /// Get the mutable storage backend
    pub fn get_storage_backend_mut(&mut self) -> Option<&mut S> {
        self.executor.storage_backend.as_mut()
    }

    /// Access storage with a closure (immutable)
    pub fn with_storage<F, R>(&self, f: F) -> Result<R, VMError>
    where
        F: FnOnce(&S) -> R,
    {
        match self.get_storage_backend() {
            Some(storage) => Ok(f(storage)),
            None => Err(VMError::StorageNotAvailable),
        }
    }

    /// Access storage with a closure (mutable)
    pub fn with_storage_mut<F, R>(&mut self, f: F) -> Result<R, VMError>
    where
        F: FnOnce(&mut S) -> R,
    {
        match self.get_storage_backend_mut() {
            Some(storage) => Ok(f(storage)),
            None => Err(VMError::StorageNotAvailable),
        }
    }

    /// Get the namespace
    pub fn get_namespace(&self) -> Option<&str> {
        Some(&self.executor.namespace)
    }

    /// Fork the VM for transaction support
    pub fn fork(&mut self) -> Result<Self, VMError> {
        let forked_executor = self.executor.fork()?;

        Ok(Self {
            stack: self.stack.clone(),
            memory: self.memory.clone(),
            executor: forked_executor,
        })
    }

    /// Commit a transaction from a forked VM
    pub fn commit_fork_transaction(&mut self) -> Result<(), VMError> {
        self.executor.commit_fork_transaction()
    }

    /// Rollback a transaction from a forked VM
    pub fn rollback_fork_transaction(&mut self) -> Result<(), VMError> {
        self.executor.rollback_fork_transaction()
    }

    /// Get the top value from the stack without popping it
    pub fn top(&self) -> Option<f64> {
        self.stack.top()
    }

    /// Pop a value from the stack
    pub fn pop_one(&mut self, op_name: &str) -> Result<f64, VMError> {
        self.stack.pop(op_name)
    }

    /// Pop two values from the stack
    pub fn pop_two(&mut self, op_name: &str) -> Result<(f64, f64), VMError> {
        self.stack.pop_two(op_name)
    }

    /// Set runtime parameters
    pub fn set_parameters(&mut self, parameters: HashMap<String, String>) -> Result<(), VMError> {
        self.memory.set_parameters(parameters);
        Ok(())
    }

    /// Get a copy of the current stack
    pub fn get_stack(&self) -> Vec<f64> {
        self.stack.get_stack()
    }

    /// Get a copy of the current memory map
    pub fn get_memory_map(&self) -> HashMap<String, f64> {
        self.memory.get_memory_map()
    }

    /// Clone the VM if possible
    pub fn try_clone(&self) -> Option<Self>
    where
        S: Clone,
    {
        // This is a shallow clone for now - we'd need to implement
        // a proper deep clone for each component if needed
        Some(Self {
            stack: self.stack.clone(),
            memory: self.memory.clone(),
            executor: VMExecution::new(), // Can't clone the executor directly due to generics
        })
    }

    /// Execute a sequence of operations
    pub fn execute(&mut self, ops: &[Op]) -> Result<(), VMError> {
        // Use internal execution implementation
        self.execute_inner(ops.to_vec())
    }

    /// Internal implementation of execute that takes ownership of the ops vector
    fn execute_inner(&mut self, ops: Vec<Op>) -> Result<(), VMError> {
        let mut loop_control = LoopControl::None;

        for op in ops {
            match op {
                Op::Push(value) => {
                    self.stack.push(value);
                }

                Op::Add => {
                    let (a, b) = self.stack.pop_two("Add")?;
                    let result = self.executor.execute_arithmetic(a, b, "add")?;
                    self.stack.push(result);
                }

                Op::Sub => {
                    let (a, b) = self.stack.pop_two("Sub")?;
                    let result = self.executor.execute_arithmetic(a, b, "sub")?;
                    self.stack.push(result);
                }

                Op::Mul => {
                    let (a, b) = self.stack.pop_two("Mul")?;
                    let result = self.executor.execute_arithmetic(a, b, "mul")?;
                    self.stack.push(result);
                }

                Op::Div => {
                    let (a, b) = self.stack.pop_two("Div")?;
                    let result = self.executor.execute_arithmetic(a, b, "div")?;
                    self.stack.push(result);
                }

                Op::Mod => {
                    let (a, b) = self.stack.pop_two("Mod")?;
                    let result = self.executor.execute_arithmetic(a, b, "mod")?;
                    self.stack.push(result);
                }

                Op::Store(name) => {
                    let value = self.stack.pop("Store")?;
                    self.memory.store(&name, value);
                }

                Op::Load(name) => {
                    let value = self.memory.load(&name)?;
                    self.stack.push(value);
                }

                Op::If {
                    condition,
                    then,
                    else_,
                } => {
                    // Execute the condition
                    self.execute_inner(condition)?;

                    // Check the result
                    let cond_result = self.stack.pop("If")?;

                    if cond_result != 0.0 {
                        // Condition is true, execute 'then' branch
                        self.execute_inner(then)?;
                    } else if let Some(else_branch) = else_ {
                        // Condition is false, execute 'else' branch if present
                        self.execute_inner(else_branch)?;
                    }
                }

                Op::Loop { count, body } => {
                    for _ in 0..count {
                        self.execute_inner(body.clone())?;

                        // Check for loop control signals
                        match loop_control {
                            LoopControl::Break => {
                                loop_control = LoopControl::None;
                                break;
                            }
                            LoopControl::Continue => {
                                loop_control = LoopControl::None;
                                continue;
                            }
                            LoopControl::None => {}
                        }
                    }
                }

                Op::While { condition, body } => {
                    loop {
                        // Evaluate condition
                        self.execute_inner(condition.clone())?;
                        let cond_result = self.stack.pop("While")?;

                        if cond_result == 0.0 {
                            // Condition is false, exit loop
                            break;
                        }

                        // Execute body
                        self.execute_inner(body.clone())?;

                        // Check for loop control signals
                        match loop_control {
                            LoopControl::Break => {
                                loop_control = LoopControl::None;
                                break;
                            }
                            LoopControl::Continue => {
                                loop_control = LoopControl::None;
                                continue;
                            }
                            LoopControl::None => {}
                        }
                    }
                }

                Op::Emit(message) => {
                    self.executor.emit(&message);
                }

                Op::Negate => {
                    let value = self.stack.pop("Negate")?;
                    self.stack.push(-value);
                }

                Op::AssertTop(expected) => {
                    let actual = self.stack.pop("AssertTop")?;
                    if (actual - expected).abs() > f64::EPSILON {
                        return Err(VMError::AssertionFailed {
                            message: format!("Expected {}, got {}", expected, actual),
                        });
                    }
                }

                Op::DumpStack => {
                    self.executor.emit(&self.stack.format_stack());
                }

                Op::DumpMemory => {
                    // Format and emit the memory state
                    let memory_str = format!("{}", self.memory);
                    self.executor.emit(&memory_str);
                }

                Op::AssertMemory { key, expected } => {
                    let actual = self.memory.load(&key)?;
                    if (actual - expected).abs() > f64::EPSILON {
                        return Err(VMError::AssertionFailed {
                            message: format!(
                                "Memory '{}': expected {}, got {}",
                                key, expected, actual
                            ),
                        });
                    }
                }

                Op::Pop => {
                    self.stack.pop("Pop")?;
                }

                Op::Eq => {
                    let (a, b) = self.stack.pop_two("Eq")?;
                    let result = self.executor.execute_comparison(a, b, "eq")?;
                    self.stack.push(result);
                }

                Op::Gt => {
                    let (a, b) = self.stack.pop_two("Gt")?;
                    let result = self.executor.execute_comparison(a, b, "gt")?;
                    self.stack.push(result);
                }

                Op::Lt => {
                    let (a, b) = self.stack.pop_two("Lt")?;
                    let result = self.executor.execute_comparison(a, b, "lt")?;
                    self.stack.push(result);
                }

                Op::Not => {
                    let value = self.stack.pop("Not")?;
                    let result = self.executor.execute_logical(value, "not")?;
                    self.stack.push(result);
                }

                Op::And => {
                    let (a, b) = self.stack.pop_two("And")?;
                    let result = self.executor.execute_binary_logical(a, b, "and")?;
                    self.stack.push(result);
                }

                Op::Or => {
                    let (a, b) = self.stack.pop_two("Or")?;
                    let result = self.executor.execute_binary_logical(a, b, "or")?;
                    self.stack.push(result);
                }

                Op::Dup => {
                    self.stack.dup("Dup")?;
                }

                Op::Swap => {
                    self.stack.swap("Swap")?;
                }

                Op::Over => {
                    self.stack.over("Over")?;
                }

                Op::Def { name, params, body } => {
                    self.memory.define_function(&name, params, body);
                }

                Op::Call(name) => {
                    self.execute_call(&name)?;
                }

                Op::Return => {
                    // If we're in a function, set the return value from the stack
                    if self.memory.in_function_call() {
                        let return_value = self.stack.top().unwrap_or(0.0);
                        self.memory.set_return_value(return_value)?;
                    }
                    // The actual return is handled in execute_call
                    break;
                }

                Op::Nop => {
                    // Do nothing
                }

                Op::Match {
                    value,
                    cases,
                    default,
                } => {
                    // Evaluate the value to match on
                    self.execute_inner(value)?;
                    let match_value = self.stack.pop("Match")?;

                    let mut matched = false;

                    // Check each case
                    for (case_value, case_body) in cases {
                        if (match_value - case_value).abs() < f64::EPSILON {
                            // Found a match, execute the corresponding body
                            self.execute_inner(case_body)?;
                            matched = true;
                            break;
                        }
                    }

                    // If no match was found and there's a default case, execute it
                    if !matched {
                        if let Some(default_body) = default {
                            self.execute_inner(default_body)?;
                        }
                    }
                }

                Op::Break => {
                    loop_control = LoopControl::Break;
                    break;
                }

                Op::Continue => {
                    loop_control = LoopControl::Continue;
                    break;
                }

                Op::EmitEvent { category, message } => {
                    self.executor.emit_event(&category, &message);
                }

                Op::AssertEqualStack { depth } => {
                    if !self.stack.assert_equal_stack(depth, "AssertEqualStack")? {
                        return Err(VMError::AssertionFailed {
                            message: format!("Stack depth {} values are not equal", depth),
                        });
                    }
                }

                Op::DumpState => {
                    // Format and emit both stack and memory state
                    self.executor.emit(&self.stack.format_stack());
                    let memory_str = format!("{}", self.memory);
                    self.executor.emit(&memory_str);
                }

                Op::CreateResource(resource) => {
                    self.executor.execute_create_resource(&resource)?;
                }

                Op::Mint {
                    resource,
                    account,
                    amount,
                    reason,
                } => {
                    self.executor
                        .execute_mint(&resource, &account, amount, &reason)?;
                }

                Op::Transfer {
                    resource,
                    from,
                    to,
                    amount,
                    reason,
                } => {
                    self.executor
                        .execute_transfer(&resource, &from, &to, amount, &reason)?;
                }

                Op::Burn {
                    resource,
                    account,
                    amount,
                    reason,
                } => {
                    self.executor
                        .execute_burn(&resource, &account, amount, &reason)?;
                }

                Op::Balance { resource, account } => {
                    let balance = self.executor.execute_balance(&resource, &account)?;
                    self.stack.push(balance);
                }

                Op::IncrementReputation {
                    identity_id,
                    amount,
                    ..
                } => {
                    self.executor
                        .execute_increment_reputation(&identity_id, amount)?;
                }

                Op::StoreP(key) => {
                    let value = self.stack.pop("StoreP")?;
                    self.executor.execute_store_p(&key, value)?;
                }

                Op::LoadP(key) => {
                    let value = self.executor.execute_load_p(&key)?;
                    self.stack.push(value);
                }

                // For other operations not yet implemented, add placeholders
                _ => {
                    return Err(VMError::NotImplemented(format!(
                        "Operation not implemented: {:?}",
                        op
                    )));
                }
            }
        }

        Ok(())
    }

    /// Execute a function call
    fn execute_call(&mut self, name: &str) -> Result<(), VMError> {
        // Retrieve the function definition
        let (params, body) = self.memory.get_function(name)?;

        // Prepare parameters from the stack
        let mut param_values = HashMap::new();

        // Pop values from the stack for each parameter (in reverse order)
        for param_name in params.iter().rev() {
            let value = self.stack.pop(&format!("Call({})", name))?;
            param_values.insert(param_name.clone(), value);
        }

        // Create a new call frame
        self.memory.push_call_frame(name, param_values);

        // Execute the function body
        self.execute_inner(body)?;

        // Pop the call frame
        let frame = self.memory.pop_call_frame().unwrap();

        // If there's a return value, push it onto the stack
        if let Some(return_value) = frame.return_value {
            self.stack.push(return_value);
        }

        Ok(())
    }

    /// Get the current output
    pub fn get_output(&self) -> &str {
        self.executor.get_output()
    }

    /// Get the current events
    pub fn get_events(&self) -> &[VMEvent] {
        self.executor.get_events()
    }
}

pub mod tests {
    use super::*;
    use crate::identity::Identity;
    use crate::storage::auth::AuthContext;
    use crate::storage::implementations::in_memory::InMemoryStorage;

    // This implementation conflicts with one in the actual InMemoryStorage module
    // Removing to avoid the conflict
    /*
    impl Default for InMemoryStorage {
        fn default() -> Self {
            Self::new()
        }
    }
    */

    #[cfg(test)]
    fn create_test_identity(id: &str, identity_type: &str) -> Identity {
        Identity::new(id.to_string(), None, identity_type.to_string(), None).unwrap()
    }

    #[cfg(test)]
    fn setup_identity_context() -> AuthContext {
        let member = create_test_identity("test_member", "member");
        let member_did = member.id.clone();

        let mut auth_ctx = AuthContext::new(&member_did);
        auth_ctx.register_identity(member);
        auth_ctx.add_role("global", "admin");
        auth_ctx.add_role("test_ns", "writer");
        auth_ctx.add_role("test_ns", "reader");

        auth_ctx
    }

    #[test]
    fn test_basic_ops() {
        let mut vm = VM::<InMemoryStorage>::new();

        // Create a simple program with arithmetic and stack operations
        let program = vec![
            Op::Push(5.0),
            Op::Push(3.0),
            Op::Add,
            Op::Push(2.0),
            Op::Mul,
        ];

        vm.execute(&program).unwrap();

        assert_eq!(vm.stack.top(), Some(16.0));
    }

    #[test]
    fn test_memory_operations() {
        let mut vm = VM::<InMemoryStorage>::new();

        // Create a program that uses memory operations
        let program = vec![
            Op::Push(42.0),
            Op::Store("answer".to_string()),
            Op::Push(7.0),
            Op::Load("answer".to_string()),
            Op::Add,
        ];

        vm.execute(&program).unwrap();

        assert_eq!(vm.stack.top(), Some(49.0));
    }

    #[test]
    fn test_function_definition_and_call() {
        let mut vm = VM::<InMemoryStorage>::new();

        // Define a function and call it
        let program = vec![
            // Define a function "add" that adds two parameters
            Op::Def {
                name: "add".to_string(),
                params: vec!["a".to_string(), "b".to_string()],
                body: vec![
                    Op::Load("a".to_string()),
                    Op::Load("b".to_string()),
                    Op::Add,
                    Op::Return,
                ],
            },
            // Call the function with arguments 5 and 3
            Op::Push(5.0),
            Op::Push(3.0),
            Op::Call("add".to_string()),
        ];

        vm.execute(&program).unwrap();

        assert_eq!(vm.stack.top(), Some(8.0));
    }

    #[test]
    fn test_conditional_execution() {
        let mut vm = VM::<InMemoryStorage>::new();

        // Test conditional execution with if/else
        let program = vec![
            Op::Push(10.0),
            Op::Push(5.0),
            Op::If {
                condition: vec![
                    Op::Push(1.0), // true condition
                ],
                then: vec![Op::Add],
                else_: Some(vec![Op::Sub]),
            },
        ];

        vm.execute(&program).unwrap();

        assert_eq!(vm.stack.top(), Some(15.0));
    }

    #[test]
    fn test_loops() {
        let mut vm = VM::<InMemoryStorage>::new();

        // Test loop operation
        let program = vec![
            Op::Push(0.0), // Initial counter
            Op::Loop {
                count: 5,
                body: vec![Op::Push(1.0), Op::Add],
            },
        ];

        vm.execute(&program).unwrap();

        assert_eq!(vm.stack.top(), Some(5.0));
    }

    #[test]
    fn test_storage_operations_mock() {
        let storage = InMemoryStorage::new();
        let auth = setup_identity_context();

        let mut vm = VM::with_storage_backend(storage);
        vm.set_auth_context(auth);
        vm.set_namespace("test_namespace");

        // Test creating a resource and minting some units
        let program = vec![
            Op::CreateResource("token".to_string()),
            Op::Mint {
                resource: "token".to_string(),
                account: "user1".to_string(),
                amount: 100.0,
                reason: Some("Initial allocation".to_string()),
            },
            Op::Balance {
                resource: "token".to_string(),
                account: "user1".to_string(),
            },
        ];

        vm.execute(&program).unwrap();

        // Check that balance query pushed the correct amount to the stack
        assert_eq!(vm.stack.top(), Some(100.0));
    }
}
