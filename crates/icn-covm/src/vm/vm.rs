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

use crate::storage::auth::AuthContext;
use crate::storage::traits::Storage;
use crate::typed::TypedValue;
use crate::vm::errors::VMError;
use crate::vm::execution::{ExecutorOps, VMExecution};
use crate::vm::memory::{MemoryScope, VMMemory};
use crate::vm::stack::{StackOps, VMStack};
use crate::vm::types::{LoopControl, Op, VMEvent};
use icn_ledger::DagLedger;

use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::{Send, Sync};
use std::path::PathBuf;

/// Defines behavior when a key is not found in storage operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MissingKeyBehavior {
    /// Return a default value (0.0) when a key is not found
    Default,
    /// Return an error when a key is not found
    Error,
}

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

    /// Behavior when a key is not found in storage
    pub missing_key_behavior: MissingKeyBehavior,

    /// DAG ledger for recording proposal lifecycle events
    pub dag: Option<DagLedger>,

    /// Whether to trace execution (print ops and stack)
    pub trace_enabled: bool,

    /// Whether to explain operations in plain English
    pub explain_enabled: bool,

    /// Whether to simulate execution (don't modify persistent storage)
    pub simulation_mode: bool,

    /// Whether to enable verbose tracing of storage operations
    pub verbose_storage_trace: bool,
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
            missing_key_behavior: MissingKeyBehavior::Default,
            dag: Some(DagLedger::new()),
            trace_enabled: false,
            explain_enabled: false,
            simulation_mode: false,
            verbose_storage_trace: false,
        }
    }

    /// Create a new VM with a storage backend
    pub fn with_storage_backend(backend: S) -> Self {
        let mut vm = Self::new();
        vm.set_storage_backend(backend);
        vm
    }

    /// Initialize the DAG ledger with a path
    pub fn with_dag_path(mut self, path: PathBuf) -> Self {
        self.dag = Some(DagLedger::with_path(path));
        self
    }

    /// Set the DAG ledger path
    pub fn set_dag_path(&mut self, path: PathBuf) -> &mut Self {
        if let Some(dag) = &mut self.dag {
            dag.set_path(path);
        } else {
            self.dag = Some(DagLedger::with_path(path));
        }
        self
    }

    /// Get the DAG ledger
    pub fn get_dag(&self) -> Option<&DagLedger> {
        self.dag.as_ref()
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

    /// Set the behavior when a key is not found in storage
    pub fn set_missing_key_behavior(&mut self, behavior: MissingKeyBehavior) {
        self.missing_key_behavior = behavior;
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
            missing_key_behavior: self.missing_key_behavior,
            dag: self.dag.clone(),
            trace_enabled: self.trace_enabled,
            explain_enabled: self.explain_enabled,
            simulation_mode: self.simulation_mode,
            verbose_storage_trace: self.verbose_storage_trace,
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

    /// Get the top value of the stack
    pub fn top(&self) -> Option<&TypedValue> {
        self.stack.top()
    }

    /// Pop one value from the stack
    pub fn pop_one(&mut self, op_name: &str) -> Result<TypedValue, VMError> {
        self.stack.pop(op_name)
    }

    /// Pop two values from the stack
    pub fn pop_two(&mut self, op_name: &str) -> Result<(TypedValue, TypedValue), VMError> {
        self.stack.pop_two(op_name)
    }

    /// Set parameters for the VM from a map of string values
    pub fn set_parameters(&mut self, parameters: HashMap<String, String>) -> Result<(), VMError> {
        for (key, value) in parameters {
            // Parse the value as a number, boolean, or keep as string
            let typed_value = if value.eq_ignore_ascii_case("true") {
                TypedValue::Boolean(true)
            } else if value.eq_ignore_ascii_case("false") {
                TypedValue::Boolean(false)
            } else if let Ok(num) = value.parse::<f64>() {
                TypedValue::Number(num)
            } else {
                TypedValue::String(value)
            };
            
            self.memory.store_param(&key, typed_value);
        }
        Ok(())
    }

    /// Get the current stack as a vector
    pub fn get_stack(&self) -> Vec<TypedValue> {
        self.stack.get_stack()
    }

    /// Get the memory map
    pub fn get_memory_map(&self) -> HashMap<String, TypedValue> {
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
            missing_key_behavior: self.missing_key_behavior,
            dag: self.dag.clone(),
            trace_enabled: self.trace_enabled,
            explain_enabled: self.explain_enabled,
            simulation_mode: self.simulation_mode,
            verbose_storage_trace: self.verbose_storage_trace,
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
            if self.trace_enabled {
                self.log_trace(&op);
            }

            if self.explain_enabled {
                self.log_explanation(&op);
            }

            // Check for simulation mode with storage operations
            match &op {
                Op::StoreP(_)
                | Op::LoadP(_)
                | Op::LoadVersionP { .. }
                | Op::ListVersionsP(_)
                | Op::DiffVersionsP { .. }
                | Op::CreateResource(_)
                | Op::Mint { .. }
                | Op::Transfer { .. }
                | Op::Burn { .. }
                | Op::Balance { .. }
                    if self.simulation_mode =>
                {
                    // In simulation mode, log the operation but don't execute storage modifications
                    self.executor
                        .emit(&format!("[SIMULATION] Would execute: {}", op));

                    // For operations that would push a value to the stack, push a placeholder
                    match &op {
                        Op::LoadP(_) | Op::LoadVersionP { .. } | Op::Balance { .. } => {
                            // Push a simulated value (0.0 for numbers)
                            // In a real implementation, you might want to be smarter about the type
                            self.stack.push(TypedValue::Number(0.0));
                        }
                        _ => {}
                    }

                    // Skip to the next operation
                    continue;
                }
                _ => {}
            }

            // Execute the operation
            match op {
                Op::Push(value) => {
                    self.stack.push(value);
                }
                Op::Add => {
                    let (a, b) = self.stack.pop_two("Add")?;
                    let result = self.executor.execute_arithmetic(&a, &b, "add")?;
                    self.stack.push(result);
                }
                Op::Sub => {
                    let (a, b) = self.stack.pop_two("Sub")?;
                    let result = self.executor.execute_arithmetic(&a, &b, "sub")?;
                    self.stack.push(result);
                }
                Op::Mul => {
                    let (a, b) = self.stack.pop_two("Mul")?;
                    let result = self.executor.execute_arithmetic(&a, &b, "mul")?;
                    self.stack.push(result);
                }
                Op::Div => {
                    let (a, b) = self.stack.pop_two("Div")?;
                    let result = self.executor.execute_arithmetic(&a, &b, "div")?;
                    self.stack.push(result);
                }
                Op::Mod => {
                    let (a, b) = self.stack.pop_two("Mod")?;
                    let result = self.executor.execute_arithmetic(&a, &b, "mod")?;
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
                    let result = self.executor.execute_comparison(&a, &b, "eq")?;
                    self.stack.push(result);
                }
                Op::Gt => {
                    let (a, b) = self.stack.pop_two("Gt")?;
                    let result = self.executor.execute_comparison(&a, &b, "gt")?;
                    self.stack.push(result);
                }
                Op::Lt => {
                    let (a, b) = self.stack.pop_two("Lt")?;
                    let result = self.executor.execute_comparison(&a, &b, "lt")?;
                    self.stack.push(result);
                }
                Op::Not => {
                    let value = self.stack.pop("Not")?;
                    let result = self.executor.execute_logical(&value, "not")?;
                    self.stack.push(result);
                }
                Op::And => {
                    let (a, b) = self.stack.pop_two("And")?;
                    let result = self.executor.execute_binary_logical(&a, &b, "and")?;
                    self.stack.push(result);
                }
                Op::Or => {
                    let (a, b) = self.stack.pop_two("Or")?;
                    let result = self.executor.execute_binary_logical(&a, &b, "or")?;
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
                    let amount_value = TypedValue::Number(*amount);
                    self.executor
                        .execute_mint(&resource, &account, &amount_value, &reason)?;
                }
                Op::Transfer {
                    resource,
                    from,
                    to,
                    amount,
                    reason,
                } => {
                    let amount_value = TypedValue::Number(*amount);
                    self.executor
                        .execute_transfer(&resource, &from, &to, &amount_value, &reason)?;
                }
                Op::Burn {
                    resource,
                    account,
                    amount,
                    reason,
                } => {
                    let amount_value = TypedValue::Number(*amount);
                    self.executor
                        .execute_burn(&resource, &account, &amount_value, &reason)?;
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
                    let amount_value = amount.map(|a| TypedValue::Number(a));
                    self.executor
                        .execute_increment_reputation(&identity_id, amount_value.as_ref())?;
                }
                Op::StoreP(key) => {
                    let value = self.stack.pop("StoreP")?;
                    self.log_storage_operation("StoreP", &key, &value);
                    self.executor.execute_store_p(&key, &value)?;
                }
                Op::LoadP(key) => {
                    let value = self
                        .executor
                        .execute_load_p(&key, self.missing_key_behavior)?;
                    self.log_storage_operation("LoadP", &key, &value);
                    self.stack.push(value);
                }
                // For other operations not yet implemented, add placeholders
                _ => {
                    // Try to handle the operation with the governance module
                    if let Some(()) = crate::governance::try_handle_governance_op(self, &op)? {
                        // If the operation was handled by the governance module, we're done
                        return Ok(());
                    }

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
        let frame = self.memory.pop_call_frame().ok_or_else(|| {
            VMError::ContextMismatch(format!(
                "Expected call frame for function '{}' but none found",
                name
            ))
        })?;

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

    /// Create a new VM with tracing enabled
    pub fn with_tracing(mut self) -> Self {
        self.trace_enabled = true;
        self
    }

    /// Enable explanation of operations
    pub fn with_explanation(mut self) -> Self {
        self.explain_enabled = true;
        self
    }

    /// Enable simulation mode (no persistent storage modifications)
    pub fn in_simulation_mode(mut self) -> Self {
        self.simulation_mode = true;
        self
    }

    /// Enable verbose storage tracing
    pub fn with_verbose_storage_trace(mut self) -> Self {
        self.verbose_storage_trace = true;
        self
    }

    /// Enable or disable tracing
    pub fn set_tracing(&mut self, enabled: bool) -> &mut Self {
        self.trace_enabled = enabled;
        self
    }

    /// Enable or disable explanation
    pub fn set_explanation(&mut self, enabled: bool) -> &mut Self {
        self.explain_enabled = enabled;
        self
    }

    /// Enable or disable simulation mode
    pub fn set_simulation_mode(&mut self, enabled: bool) -> &mut Self {
        self.simulation_mode = enabled;
        self
    }

    /// Enable or disable verbose storage tracing
    pub fn set_verbose_storage_trace(&mut self, enabled: bool) -> &mut Self {
        self.verbose_storage_trace = enabled;
        self
    }

    /// Check if verbose storage tracing is enabled
    pub fn is_verbose_storage_tracing(&self) -> bool {
        self.verbose_storage_trace
    }

    /// Check if tracing is enabled
    pub fn is_tracing(&self) -> bool {
        self.trace_enabled
    }

    /// Check if explanation is enabled
    pub fn is_explaining(&self) -> bool {
        self.explain_enabled
    }

    /// Check if simulation mode is enabled
    pub fn is_simulation_mode(&self) -> bool {
        self.simulation_mode
    }

    /// Log a trace message if tracing is enabled
    fn log_trace(&mut self, op: &Op) {
        if self.trace_enabled {
            self.executor.emit(&format!("[TRACE] Op: {}", op));

            // Only show stack state if there are items on the stack
            if !self.stack.is_empty() {
                self.executor
                    .emit(&format!("[TRACE] Stack: {:?}", self.stack.get_stack()));
            }
        }
    }

    /// Log an explanation if explanation is enabled
    fn log_explanation(&mut self, op: &Op) {
        if self.explain_enabled {
            let explanation = self.explain_op(op);
            self.executor.emit(&format!("[EXPLAIN] {}", explanation));
        }
    }

    /// Log a storage operation with tracing information
    fn log_storage_operation(&mut self, operation: &str, key: &str, value: &TypedValue) {
        if self.verbose_storage_trace {
            let value_str = match value {
                TypedValue::Number(n) => n.to_string(),
                TypedValue::Boolean(b) => b.to_string(),
                TypedValue::String(s) => format!("\"{}\"", s),
                TypedValue::Null => "null".to_string(),
            };
            
            self.executor.emit_event(
                "storage_trace",
                &format!("{} {} = {}", operation, key, value_str),
            );
        }
    }

    /// Generate an explanation for an operation
    fn explain_op(&self, op: &Op) -> String {
        match op {
            Op::Push(val) => format!("Push the value {:?} onto the stack", val),
            Op::Add => "Add the top two values on the stack".into(),
            Op::Sub => "Subtract the top value from the second value on the stack".into(),
            Op::Mul => "Multiply the top two values on the stack".into(),
            Op::Div => "Divide the second value by the top value on the stack".into(),
            Op::Mod => {
                "Compute the remainder when dividing the second value by the top value".into()
            }
            Op::Store(name) => format!("Store the top stack value in memory under '{}'", name),
            Op::Load(name) => format!(
                "Load the value of '{}' from memory and push it onto the stack",
                name
            ),
            Op::If { .. } => "Execute code conditionally based on a value".into(),
            Op::Loop { count, .. } => format!("Execute a block of code {} times", count),
            Op::While { .. } => "Execute a block of code while a condition is true".into(),
            Op::Emit(msg) => format!("Output the message: {}", msg),
            Op::Negate => "Negate the top value on the stack".into(),
            Op::AssertTop(val) => format!("Assert that the top value equals {:?}", val),
            Op::DumpStack => "Display the current stack contents".into(),
            Op::DumpMemory => "Display the current memory contents".into(),
            Op::AssertMemory { key, expected } => {
                format!("Assert that memory value '{}' equals {:?}", key, expected)
            }
            Op::Pop => "Remove the top value from the stack".into(),
            Op::Eq => "Check if the top two values are equal".into(),
            Op::Gt => "Check if the second value is greater than the top value".into(),
            Op::Lt => "Check if the second value is less than the top value".into(),
            Op::Not => "Logical NOT of the top value".into(),
            Op::And => "Logical AND of the top two values".into(),
            Op::Or => "Logical OR of the top two values".into(),
            Op::Dup => "Duplicate the top value on the stack".into(),
            Op::Swap => "Swap the top two values on the stack".into(),
            Op::Over => "Copy the second value to the top of the stack".into(),
            Op::Def { name, .. } => format!("Define a function named '{}'", name),
            Op::Call(name) => format!("Call the function named '{}'", name),
            Op::Return => "Return from the current function".into(),
            Op::Nop => "No operation (do nothing)".into(),
            Op::Match { .. } => "Match a value against several cases".into(),
            Op::Break => "Break out of the innermost loop".into(),
            Op::Continue => "Continue to the next iteration of the innermost loop".into(),
            Op::EmitEvent { category, message } => format!(
                "Emit an event with category '{}' and message '{}'",
                category, message
            ),
            Op::AssertEqualStack { depth } => format!(
                "Assert that the top {} values on the stack are equal",
                depth
            ),
            Op::DumpState => "Display the entire VM state".into(),
            Op::StoreP(key) => format!(
                "Store the top stack value in persistent storage under key '{}'",
                key
            ),
            Op::LoadP(key) => format!(
                "Load value from persistent storage with key '{}' and push it onto the stack",
                key
            ),
            _ => format!("Execute operation: {}", op),
        }
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
        // This is only used in tests, so we can use expect() with context
        Identity::new(id.to_string(), None, identity_type.to_string(), None)
            .expect("Failed to create test identity")
    }

    #[cfg(test)]
    fn setup_identity_context() -> AuthContext {
        let member = create_test_identity("test_member", "member");
        let member_did = member.did.clone();

        let mut auth_ctx = AuthContext::new(&member_did);
        auth_ctx.register_identity(member);
        auth_ctx.add_role("global", "admin");
        auth_ctx.add_role("test_ns", "writer");
        auth_ctx.add_role("test_ns", "reader");

        auth_ctx
    }

    #[test]
    fn test_basic_arithmetic() {
        let mut vm = VM::<InMemoryStorage>::default();
        
        let program = vec![
            Op::Push(TypedValue::Number(5.0)),
            Op::Push(TypedValue::Number(3.0)),
            Op::Add,
        ];
        
        vm.execute_program(&program).unwrap();
        
        // Verify stack
        assert_eq!(vm.stack.top(), Some(&TypedValue::Number(8.0)));
    }

    #[test]
    fn test_conditional_branch() {
        let mut vm = VM::<InMemoryStorage>::default();
        
        let program = vec![
            Op::Push(TypedValue::Number(10.0)),
            Op::Push(TypedValue::Number(5.0)),
            Op::IfBlock {
                condition: vec![Op::Push(TypedValue::Number(1.0)), // true condition
                ],
                body: vec![Op::Add],
                else_body: vec![Op::Sub],
            },
        ];
        
        vm.execute_program(&program).unwrap();
        
        // Stack should have 15.0 (10+5) if condition is true
        assert_eq!(vm.stack.top(), Some(&TypedValue::Number(15.0)));
    }

    #[test]
    fn test_loop() {
        let mut vm = VM::<InMemoryStorage>::default();
        
        let program = vec![
            Op::Push(TypedValue::Number(0.0)), // Initial counter
            Op::Loop {
                max_iterations: 5,
                body: vec![Op::Push(TypedValue::Number(1.0)), Op::Add],
                condition: vec![],
            },
        ];
        
        vm.execute_program(&program).unwrap();
        
        // After 5 iterations of adding 1, we should have 5
        assert_eq!(vm.stack.top(), Some(&TypedValue::Number(5.0)));
    }

    #[test]
    fn test_memory_operations() {
        let mut vm = VM::<InMemoryStorage>::new();
        
        let ops = vec![
            Op::Push(TypedValue::Number(42.0)),
            Op::Store("answer".to_string()),
            Op::Push(TypedValue::Number(7.0)),
            Op::Load("answer".to_string()),
            Op::Mul,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        
        let stack_value = vm.stack.top().unwrap();
        if let TypedValue::Number(n) = stack_value {
            assert!((n - 294.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected number on stack");
        }
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
            Op::Push(TypedValue::Number(5.0)),
            Op::Push(TypedValue::Number(3.0)),
            Op::Call("add".to_string()),
        ];

        vm.execute(&program).unwrap();

        assert_eq!(vm.stack.top(), Some(8.0));
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
