use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use crate::events::Event;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum VMError {
    #[error("Stack underflow: {op} needs {needed} values but found {found}")]
    StackUnderflow { op: String, needed: usize, found: usize },
    
    #[error("Division by zero")]
    DivisionByZero,
    
    #[error("Modulo by zero")]
    ModuloByZero,
    
    #[error("Variable '{0}' not found")]
    VariableNotFound(String),
    
    #[error("Function '{0}' not found")]
    FunctionNotFound(String),
    
    #[error("Invalid condition: {0}")]
    InvalidCondition(String),
    
    #[error("Assertion failed: expected {expected}, found {found}")]
    AssertionFailed { expected: f64, found: f64 },
    
    #[error("Assertion failed: expected {key} = {expected}, found {found}")]
    AssertMemoryFailed { key: String, expected: f64, found: f64 },
    
    #[error("AssertEqualStack failed: values at positions {index1} and {index2} differ ({val1} != {val2})")]
    AssertEqualStackFailed { index1: usize, index2: usize, val1: f64, val2: f64 },
    
    #[error("Call frame error: {0}")]
    CallFrameError(String),
    
    #[error("Maximum recursion depth exceeded")]
    MaxRecursionDepth,
    
    #[error("Assertion failed: value mismatch (expected {expected}, found {found})")]
    AssertTopMismatch { expected: f64, found: f64 },
    
    #[error("IO error: {0}")]
    IOError(String),
    
    #[error("REPL error: {0}")]
    ReplError(String),
    
    #[error("Parameter error: {0}")]
    ParameterError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Op {
    Push(f64),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Store(String),
    Load(String),
    If { condition: Vec<Op>, then: Vec<Op>, else_: Option<Vec<Op>> },
    Loop { count: usize, body: Vec<Op> },
    While { condition: Vec<Op>, body: Vec<Op> },
    Emit(String),
    Negate,
    AssertTop(f64),
    DumpStack,
    DumpMemory,
    AssertMemory { key: String, expected: f64 },
    Pop,
    Eq,
    Gt,
    Lt,
    Not,
    And,
    Or,
    Dup,
    Swap,
    Over,
    Def { name: String, params: Vec<String>, body: Vec<Op> },
    Call(String),
    Return,
    Nop,
    // New governance-inspired opcodes
    Match { value: Vec<Op>, cases: Vec<(f64, Vec<Op>)>, default: Option<Vec<Op>> },
    Break,
    Continue,
    EmitEvent { category: String, message: String },
    AssertEqualStack { depth: usize },
}

#[derive(Debug)]
struct CallFrame {
    memory: HashMap<String, f64>,
    return_value: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LoopControl {
    None,
    Break,
    Continue,
}

#[derive(Debug)]
pub struct VM {
    pub stack: Vec<f64>,
    memory: HashMap<String, f64>,
    functions: HashMap<String, (Vec<String>, Vec<Op>)>,
    call_frames: Vec<CallFrame>,
    recursion_depth: usize,
    loop_control: LoopControl,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            memory: HashMap::new(),
            functions: HashMap::new(),
            call_frames: Vec::new(),
            recursion_depth: 0,
            loop_control: LoopControl::None,
        }
    }

    pub fn get_stack(&self) -> &[f64] {
        &self.stack
    }

    pub fn get_memory(&self, key: &str) -> Option<f64> {
        self.memory.get(key).copied()
    }

    pub fn get_memory_map(&self) -> &HashMap<String, f64> {
        &self.memory
    }

    pub fn set_parameters(&mut self, params: HashMap<String, String>) -> Result<(), VMError> {
        for (key, value) in params {
            // Try to parse as f64 first
            match value.parse::<f64>() {
                Ok(num) => {
                    self.memory.insert(key.clone(), num);
                }
                Err(_) => {
                    // For non-numeric strings, we'll store the length as a numeric value
                    // This allows parameters to be used in the stack machine
                    self.memory.insert(key.clone(), value.len() as f64);
                    
                    // Also log this for debugging
                    let event = Event::info(
                        "params", 
                        &format!("Parameter '{}' is not numeric, storing length {}", key, value.len())
                    );
                    event.emit().map_err(|e| VMError::IOError(e.to_string()))?;
                }
            }
        }
        Ok(())
    }

    pub fn execute(&mut self, ops: &[Op]) -> Result<(), VMError> {
        if self.recursion_depth > 1000 {
            return Err(VMError::MaxRecursionDepth);
        }
        self.execute_inner(ops)
    }

    // Helper for stack operations that need to pop one value
    fn pop_one(&mut self, op_name: &str) -> Result<f64, VMError> {
        self.stack.pop().ok_or_else(|| VMError::StackUnderflow { 
            op: op_name.to_string(), 
            needed: 1, 
            found: 0 
        })
    }

    // Helper for stack operations that need to pop two values
    fn pop_two(&mut self, op_name: &str) -> Result<(f64, f64), VMError> {
        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow { 
                op: op_name.to_string(), 
                needed: 2, 
                found: self.stack.len() 
            });
        }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        Ok((a, b))
    }

    fn execute_inner(&mut self, ops: &[Op]) -> Result<(), VMError> {
        if self.recursion_depth > 1000 {
            return Err(VMError::MaxRecursionDepth);
        }

        let mut pc = 0;
        while pc < ops.len() {
            // Check for loop control flow
            if self.loop_control != LoopControl::None {
                break;
            }
            
            let op = &ops[pc];
            match op {
                Op::Push(value) => {
                    self.stack.push(*value);
                }
                Op::Pop => {
                    self.pop_one("Pop")?;
                }
                Op::Dup => {
                    let value = self.stack.last().ok_or_else(|| VMError::StackUnderflow { 
                        op: "Dup".to_string(), 
                        needed: 1, 
                        found: 0 
                    })?;
                    self.stack.push(*value);
                }
                Op::Swap => {
                    if self.stack.len() < 2 {
                        return Err(VMError::StackUnderflow { 
                            op: "Swap".to_string(), 
                            needed: 2, 
                            found: self.stack.len()
                        });
                    }
                    let len = self.stack.len();
                    self.stack.swap(len - 1, len - 2);
                }
                Op::Over => {
                    if self.stack.len() < 2 {
                        return Err(VMError::StackUnderflow { 
                            op: "Over".to_string(), 
                            needed: 2, 
                            found: self.stack.len()
                        });
                    }
                    let value = self.stack[self.stack.len() - 2];
                    self.stack.push(value);
                }
                Op::Emit(msg) => {
                    let event = Event::info("emit", msg);
                    event.emit().map_err(|e| VMError::IOError(e.to_string()))?;
                }
                Op::DumpStack => {
                    let stack_data = serde_json::to_value(&self.stack).unwrap_or(serde_json::Value::Null);
                    let event = Event::info("stack", "Dumping stack contents")
                        .with_data(stack_data);
                    event.emit().map_err(|e| VMError::IOError(e.to_string()))?;
                }
                Op::Def { name, params, body } => {
                    self.functions.insert(name.clone(), (params.clone(), body.clone()));
                }
                
                Op::Loop { count, body } => {
                    for i in 0..*count {
                        self.execute_inner(body)?;
                        
                        // Handle loop control flow
                        match self.loop_control {
                            LoopControl::Break => {
                                self.loop_control = LoopControl::None;
                                break;
                            },
                            LoopControl::Continue => {
                                self.loop_control = LoopControl::None;
                                continue;
                            },
                            LoopControl::None => {}
                        }
                    }
                }
                
                Op::While { condition, body } => {
                    loop {
                        self.execute_inner(condition)?;
                        
                        let cond = self.pop_one("While condition")?;
                        
                        if cond == 0.0 {
                            break;
                        }
                        
                        self.execute_inner(body)?;
                        
                        // Handle loop control flow
                        match self.loop_control {
                            LoopControl::Break => {
                                self.loop_control = LoopControl::None;
                                break;
                            },
                            LoopControl::Continue => {
                                self.loop_control = LoopControl::None;
                                continue;
                            },
                            LoopControl::None => {}
                        }
                    }
                }
                
                Op::Break => {
                    self.loop_control = LoopControl::Break;
                }
                
                Op::Continue => {
                    self.loop_control = LoopControl::Continue;
                }
                
                Op::EmitEvent { category, message } => {
                    let event = Event::info(category.as_str(), message.as_str());
                    event.emit().map_err(|e| VMError::IOError(e.to_string()))?;
                }
                
                Op::AssertEqualStack { depth } => {
                    if self.stack.len() < *depth {
                        return Err(VMError::StackUnderflow { 
                            op: "AssertEqualStack".to_string(), 
                            needed: *depth, 
                            found: self.stack.len()
                        });
                    }
                    
                    let stack_len = self.stack.len();
                    let last_value = self.stack[stack_len - 1];
                    
                    for i in 1..*depth {
                        if self.stack[stack_len - 1 - i] != last_value {
                            return Err(VMError::AssertEqualStackFailed { 
                                index1: stack_len - 1, 
                                index2: stack_len - 1 - i, 
                                val1: last_value, 
                                val2: self.stack[stack_len - 1 - i]
                            });
                        }
                    }
                }
                
                Op::Match { value, cases, default } => {
                    // Execute value operation to get the match value
                    self.execute_inner(value)?;
                    
                    let match_value = self.pop_one("Match value")?;
                    
                    // Find and execute matching case
                    let mut found_match = false;
                    for (case_value, case_ops) in cases {
                        if *case_value == match_value {
                            self.execute_inner(case_ops)?;
                            found_match = true;
                            break;
                        }
                    }
                    
                    // Execute default if no match found
                    if !found_match {
                        if let Some(default_ops) = default {
                            self.execute_inner(default_ops)?;
                        } else {
                            // No default, push the match value back
                            self.stack.push(match_value);
                        }
                    }
                }
                
                Op::Add => {
                    let (a, b) = self.pop_two("Add")?;
                    self.stack.push(a + b);
                }
                
                Op::Sub => {
                    let (a, b) = self.pop_two("Sub")?;
                    self.stack.push(a - b);
                }
                
                Op::Mul => {
                    let (a, b) = self.pop_two("Mul")?;
                    self.stack.push(a * b);
                }
                
                Op::Div => {
                    let (a, b) = self.pop_two("Div")?;
                    if b == 0.0 {
                        return Err(VMError::DivisionByZero);
                    }
                    self.stack.push(a / b);
                }
                
                Op::Mod => {
                    let (a, b) = self.pop_two("Mod")?;
                    if b == 0.0 {
                        return Err(VMError::ModuloByZero);
                    }
                    self.stack.push(a % b);
                }
                
                Op::Lt => {
                    let (a, b) = self.pop_two("Lt")?;
                    self.stack.push(if a < b { 1.0 } else { 0.0 });
                }
                
                Op::Gt => {
                    let (a, b) = self.pop_two("Gt")?;
                    self.stack.push(if a > b { 1.0 } else { 0.0 });
                }
                
                Op::Eq => {
                    let (a, b) = self.pop_two("Eq")?;
                    self.stack.push(if a == b { 1.0 } else { 0.0 });
                }
                
                Op::If { condition, then, else_ } => {
                    // Get condition value 
                    let condition_value = if condition.is_empty() {
                        // If condition is empty, use the value already on the stack
                        self.pop_one("If condition")?
                    } else {
                        // Save the stack size before executing the condition
                        let stack_size_before = self.stack.len();
                        
                        // Execute the condition operations
                        self.execute_inner(condition)?;
                        
                        // Make sure the stack has at least one more value than before
                        if self.stack.len() <= stack_size_before {
                            return Err(VMError::InvalidCondition(
                                "Condition block did not leave a value on the stack".to_string()
                            ));
                        }
                        
                        // Get the top value from the stack
                        self.pop_one("If condition result")?
                    };

                    // Execute the then block when condition is 0.0 (true)
                    // or the else block when condition is non-zero (false)
                    if condition_value == 0.0 {
                        self.execute_inner(then)?;
                    } else if let Some(else_block) = else_ {
                        self.execute_inner(else_block)?;
                    } else {
                        // If condition is non-zero (false) and no else block, preserve the condition value
                        self.stack.push(condition_value);
                    }
                }
                
                Op::Negate => {
                    let value = self.pop_one("Negate")?;
                    self.stack.push(-value);
                }
                
                Op::Not => {
                    let value = self.pop_one("Not")?;
                    self.stack.push(if value == 0.0 { 1.0 } else { 0.0 });
                }
                
                Op::And => {
                    let (a, b) = self.pop_two("And")?;
                    self.stack.push(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 });
                }
                
                Op::Or => {
                    let (a, b) = self.pop_two("Or")?;
                    self.stack.push(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 });
                }
                
                Op::AssertTop(expected) => {
                    let value = self.stack.last().ok_or_else(|| VMError::StackUnderflow { 
                        op: "AssertTop".to_string(), 
                        needed: 1, 
                        found: 0 
                    })?;
                    
                    if *value != *expected {
                        return Err(VMError::AssertTopMismatch { 
                            expected: *expected, 
                            found: *value 
                        });
                    }
                }
                
                Op::DumpMemory => {
                    let memory_data = serde_json::to_value(&self.memory).unwrap_or(serde_json::Value::Null);
                    let event = Event::info("memory", "Dumping memory contents")
                        .with_data(memory_data);
                    event.emit().map_err(|e| VMError::IOError(e.to_string()))?;
                }
                
                Op::AssertMemory { key, expected } => {
                    let value = self.memory.get(key)
                        .ok_or_else(|| VMError::VariableNotFound(key.clone()))?;
                    
                    if *value != *expected {
                        return Err(VMError::AssertMemoryFailed { 
                            key: key.clone(), 
                            expected: *expected, 
                            found: *value 
                        });
                    }
                }
                
                Op::Nop => {
                    // Do nothing for Nop
                }
                
                Op::Load(name) => {
                    let value = self.memory.get(name)
                        .ok_or_else(|| VMError::VariableNotFound(name.clone()))?;
                    self.stack.push(*value);
                }
                
                Op::Store(name) => {
                    let value = self.pop_one("Store")?;
                    self.memory.insert(name.clone(), value);
                }
                
                Op::Call(name) => {
                    let (params, body) = self.functions.get(name)
                        .ok_or_else(|| VMError::FunctionNotFound(name.clone()))?.clone();
                    
                    if self.stack.len() < params.len() {
                        let missing_param = &params[self.stack.len()];
                        return Err(VMError::StackUnderflow { 
                            op: format!("Call to function '{}'", name), 
                            needed: params.len(), 
                            found: self.stack.len() 
                        });
                    }
                    
                    let mut local_memory = HashMap::new();
                    let mut args_to_pop = params.len();
                    let mut temp_args = Vec::with_capacity(args_to_pop);
                    
                    while args_to_pop > 0 {
                        temp_args.push(self.stack.pop().unwrap());
                        args_to_pop -= 1;
                    }
                    
                    for (param_name, arg_value) in params.iter().zip(temp_args.into_iter().rev()) {
                        local_memory.insert(param_name.clone(), arg_value);
                    }
                    
                    self.call_frames.push(CallFrame {
                        memory: std::mem::replace(&mut self.memory, local_memory),
                        return_value: None,
                    });
                    
                    self.recursion_depth += 1;
                    let result = self.execute_inner(&body);
                    self.recursion_depth -= 1;
                    
                    let frame = self.call_frames.pop().ok_or_else(|| 
                        VMError::CallFrameError("Call frame stack underflow (internal error)".to_string())
                    )?;
                    
                    let return_value = frame.return_value;
                    self.memory = frame.memory;
                    
                    if let Some(val) = return_value {
                        self.stack.push(val);
                    }
                    
                    result?;
                }
                
                Op::Return => {
                    let return_value = self.stack.pop();
                    
                    if let Some(frame) = self.call_frames.last_mut() {
                        frame.return_value = return_value;
                    } else {
                        return Ok(());
                    }
                    
                    return Ok(());
                }
            }
            pc += 1;
        }
        Ok(())
    }

    // These methods are used in tests
    #[cfg(test)]
    pub fn top(&self) -> Option<f64> {
        self.stack.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};
    use std::sync::Mutex;

    #[test]
    fn test_basic_arithmetic() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0),
            Op::Push(3.0),
            Op::Add,
            Op::Push(2.0),
            Op::Mul,
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(16.0));
    }

    #[test]
    fn test_division() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(10.0), Op::Push(2.0), Op::Div];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(5.0));
    }

    #[test]
    fn test_division_by_zero() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(10.0), Op::Push(0.0), Op::Div];

        assert_eq!(vm.execute(&ops), Err(VMError::DivisionByZero));
    }

    #[test]
    fn test_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(5.0), Op::Add];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow { 
                op: "Add".to_string(), 
                needed: 2, 
                found: 1 
            })
        );
    }

    #[test]
    fn test_store_and_load() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Store("x".to_string()),
            Op::Load("x".to_string()),
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
        assert_eq!(vm.get_memory("x"), Some(42.0));
    }

    #[test]
    fn test_load_nonexistent() {
        let mut vm = VM::new();
        let ops = vec![Op::Load("nonexistent".to_string())];

        assert_eq!(vm.execute(&ops), Err(VMError::VariableNotFound("nonexistent".to_string())));
    }

    #[test]
    fn test_store_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Store("x".to_string())];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow { 
                op: "Store".to_string(), 
                needed: 1, 
                found: 0 
            })
        );
    }

    #[test]
    fn test_memory_arithmetic() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(10.0),
            Op::Store("x".to_string()),
            Op::Push(5.0),
            Op::Store("y".to_string()),
            Op::Load("x".to_string()),
            Op::Load("y".to_string()),
            Op::Add,
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(15.0));
        assert_eq!(vm.get_memory("x"), Some(10.0));
        assert_eq!(vm.get_memory("y"), Some(5.0));
    }

    #[test]
    fn test_if_zero_true() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),  // Condition value is 0.0 (true in this VM)
            Op::If {
                condition: vec![],
                then: vec![Op::Push(42.0)],  // Should execute when condition is 0.0
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));  // Then block executed because condition was 0.0 (true)
    }

    #[test]
    fn test_if_zero_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0),  // Condition value is non-zero (false in this VM)
            Op::If {
                condition: vec![],
                then: vec![Op::Push(42.0)],  // Should not execute
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));  // Then block not executed, original value remains
    }

    #[test]
    fn test_if_zero_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::If {
            condition: vec![],
            then: vec![Op::Push(42.0)],
            else_: None,
        }];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow { 
                op: "If".to_string(), 
                needed: 1, 
                found: 0 
            })
        );
    }

    #[test]
    fn test_nested_if_zero() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),  // Initial stack value (true)
            Op::If {
                condition: vec![
                    Op::Push(1.0),  // Push false for outer condition
                    Op::If {
                        condition: vec![Op::Push(0.0)],  // Push true for inner condition
                        then: vec![Op::Push(42.0)],      // Should run (condition is true/0.0)
                        else_: None,
                    },
                ],
                then: vec![Op::Push(24.0)],  // This should run if the condition evaluates to 0.0
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        
        // The outer condition operation pushes 1.0 and then contains a nested if 
        // that leaves 42.0 on the stack. So the condition is 42.0, not 0.0, 
        // meaning the then block should not run, leaving 42.0 as the final result.
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_loop_basic() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Store("counter".to_string()),
            Op::Loop {
                count: 3,
                body: vec![
                    Op::Load("counter".to_string()),
                    Op::Push(1.0),
                    Op::Add,
                    Op::Store("counter".to_string()),
                ],
            },
            Op::Load("counter".to_string()),
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(3.0));
    }

    #[test]
    fn test_loop_zero() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Store("value".to_string()),
            Op::Loop {
                count: 0,
                body: vec![Op::Push(100.0), Op::Store("value".to_string())],
            },
            Op::Load("value".to_string()),
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_nested_loops() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Store("outer".to_string()),
            Op::Push(0.0),
            Op::Store("inner".to_string()),
            Op::Loop {
                count: 2,
                body: vec![
                    Op::Load("outer".to_string()),
                    Op::Push(1.0),
                    Op::Add,
                    Op::Store("outer".to_string()),
                    Op::Loop {
                        count: 3,
                        body: vec![
                            Op::Load("inner".to_string()),
                            Op::Push(1.0),
                            Op::Add,
                            Op::Store("inner".to_string()),
                        ],
                    },
                ],
            },
            Op::Load("outer".to_string()),
            Op::Load("inner".to_string()),
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.get_memory("outer"), Some(2.0));
        assert_eq!(vm.get_memory("inner"), Some(6.0));
    }

    #[test]
    fn test_loop_with_arithmetic() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0),
            Op::Store("result".to_string()),
            Op::Loop {
                count: 4,
                body: vec![
                    Op::Load("result".to_string()),
                    Op::Push(2.0),
                    Op::Mul,
                    Op::Store("result".to_string()),
                ],
            },
            Op::Load("result".to_string()),
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(16.0)); // 1 * 2^4
    }

    #[test]
    fn test_emit() {
        let mut vm = VM::new();
        let ops = vec![Op::Emit("Test message".to_string())];

        assert!(vm.execute(&ops).is_ok());
    }

    #[test]
    fn test_emit_with_arithmetic() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0),
            Op::Push(3.0),
            Op::Add,
            Op::Emit("Result:".to_string()),
            Op::Store("result".to_string()),
            Op::Load("result".to_string()),
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(8.0));
    }

    #[test]
    fn test_emit_in_loop() {
        let mut vm = VM::new();
        let ops = vec![Op::Loop {
            count: 3,
            body: vec![Op::Emit("Loop iteration".to_string())],
        }];

        assert!(vm.execute(&ops).is_ok());
    }

    #[test]
    fn test_negate() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Negate,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(-42.0));
    }

    #[test]
    fn test_negate_zero() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Negate,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_negate_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Negate];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "Negate".to_string(), 
            needed: 1, 
            found: 0 
        }));
    }

    #[test]
    fn test_negate_with_arithmetic() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0),
            Op::Push(3.0),
            Op::Add,
            Op::Negate,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(-8.0));
    }

    #[test]
    fn test_assert_top_success() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::AssertTop(42.0),
        ];
        
        assert!(vm.execute(&ops).is_ok());
    }

    #[test]
    fn test_assert_top_failure() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::AssertTop(24.0),
        ];
        
        assert_eq!(vm.execute(&ops), Err(VMError::AssertTopMismatch { 
            expected: 24.0, 
            found: 42.0 
        }));
    }

    #[test]
    fn test_assert_top_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::AssertTop(42.0)];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "AssertTop".to_string(), 
            needed: 1, 
            found: 0 
        }));
    }

    #[test]
    fn test_assert_top_with_arithmetic() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0),
            Op::Push(3.0),
            Op::Add,
            Op::AssertTop(8.0),
        ];
        
        assert!(vm.execute(&ops).is_ok());
    }

    #[test]
    fn test_dump_stack() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0),
            Op::Push(2.0),
            Op::Push(3.0),
            Op::DumpStack,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_dump_memory() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Store("x".to_string()),
            Op::Push(24.0),
            Op::Store("y".to_string()),
            Op::DumpMemory,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.get_memory("x"), Some(42.0));
        assert_eq!(vm.get_memory("y"), Some(24.0));
    }

    #[test]
    fn test_dump_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::DumpStack];
        
        assert!(vm.execute(&ops).is_ok());
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn test_dump_empty_memory() {
        let mut vm = VM::new();
        let ops = vec![Op::DumpMemory];
        
        assert!(vm.execute(&ops).is_ok());
        assert!(vm.memory.is_empty());
    }

    #[test]
    fn test_logic_not_true() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Not,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_not_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Not,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_not_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Not];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "Not".to_string(), 
            needed: 1, 
            found: 0 
        }));
    }

    #[test]
    fn test_logic_and_true_true() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Push(24.0),
            Op::And,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_and_true_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Push(0.0),
            Op::And,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_and_false_true() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Push(42.0),
            Op::And,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_and_false_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Push(0.0),
            Op::And,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_and_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::And];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "And".to_string(), 
            needed: 2, 
            found: 1 
        }));
    }

    #[test]
    fn test_logic_or_true_true() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Push(24.0),
            Op::Or,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_or_true_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Push(0.0),
            Op::Or,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_or_false_true() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Push(42.0),
            Op::Or,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_or_false_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Push(0.0),
            Op::Or,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_or_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Or];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "Or".to_string(), 
            needed: 2, 
            found: 1 
        }));
    }

    #[test]
    fn test_while_countdown() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0),
            Op::Store("counter".to_string()),
            Op::While {
                condition: vec![
                    Op::Load("counter".to_string()),
                    Op::Push(0.0),
                    Op::Gt,
                ],
                body: vec![
                    Op::Load("counter".to_string()),
                    Op::Push(1.0),
                    Op::Sub,
                    Op::Store("counter".to_string()),
                ],
            },
            Op::Load("counter".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_while_empty_condition() {
        let mut vm = VM::new();
        let ops = vec![Op::While {
            condition: vec![],
            body: vec![Op::Push(42.0)],
        }];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "While".to_string(), 
            needed: 1, 
            found: 0 
        }));
    }

    #[test]
    fn test_while_zero_condition() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Store("counter".to_string()),
            Op::While {
                condition: vec![Op::Load("counter".to_string())],
                body: vec![Op::Push(42.0)],
            },
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.get_memory("counter"), Some(0.0));
    }

    #[test]
    fn test_stack_dup() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Dup,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![42.0, 42.0]);
    }

    #[test]
    fn test_stack_dup_empty() {
        let mut vm = VM::new();
        let ops = vec![Op::Dup];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "Dup".to_string(), 
            needed: 1, 
            found: 0 
        }));
    }

    #[test]
    fn test_stack_swap() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Push(24.0),
            Op::Swap,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![24.0, 42.0]);
    }

    #[test]
    fn test_stack_swap_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Swap];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "Swap".to_string(), 
            needed: 2, 
            found: 1 
        }));
    }

    #[test]
    fn test_stack_over() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Push(24.0),
            Op::Over,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![42.0, 24.0, 42.0]);
    }

    #[test]
    fn test_stack_over_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Over];
        
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "Over".to_string(), 
            needed: 2, 
            found: 1 
        }));
    }

    #[test]
    fn test_stack_manipulation_chain() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0),
            Op::Push(2.0),
            Op::Push(3.0),
            Op::Dup,   // Stack: [1, 2, 3, 3]
            Op::Swap,  // Stack: [1, 2, 3, 3] -> [1, 2, 3, 3]
            Op::Over,  // Stack: [1, 2, 3, 3, 3]
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![1.0, 2.0, 3.0, 3.0, 3.0]);
    }

    #[test]
    fn test_function_definition_and_call() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "double".to_string(),
                params: vec![],
                body: vec![
                    Op::Push(2.0),
                    Op::Mul,
                ],
            },
            Op::Push(21.0),
            Op::Call("double".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_function_not_found() {
        let mut vm = VM::new();
        let ops = vec![Op::Call("nonexistent".to_string())];
        
        assert_eq!(vm.execute(&ops), Err(VMError::FunctionNotFound("nonexistent".to_string())));
    }

    #[test]
    fn test_function_return() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "add_one".to_string(),
                params: vec![],
                body: vec![
                    Op::Push(1.0),
                    Op::Add,
                    Op::Return,
                ],
            },
            Op::Push(41.0),
            Op::Call("add_one".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_function_with_memory() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "store_and_load".to_string(),
                params: vec![],
                body: vec![
                    Op::Store("x".to_string()),
                    Op::Load("x".to_string()),
                    Op::Return,
                ],
            },
            Op::Push(42.0),
            Op::Call("store_and_load".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_recursive_function() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "countdown".to_string(),
                params: vec![],
                body: vec![
                    Op::Dup,  // Duplicate the value for comparison
                    Op::Push(0.0),
                    Op::Eq,   // Will push 1.0 if n==0, 0.0 otherwise
                    Op::If {
                        condition: vec![],
                        then: vec![
                            // Already 0, just return
                            Op::Push(0.0),  // Explicitly push 0 for the result
                        ],
                        else_: Some(vec![
                            // n > 0, so decrement and recurse
                            Op::Push(1.0),
                            Op::Sub,      // Decrement n
                            Op::Call("countdown".to_string()),  // Recursive call
                        ]),
                    },
                ],
            },
            Op::Push(3.0),  // Use a smaller number to avoid stack overflow
            Op::Call("countdown".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_function_stack_isolation() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "push_and_pop".to_string(),
                params: vec![],
                body: vec![
                    Op::Push(42.0),
                    Op::Pop,
                    Op::Return,
                ],
            },
            Op::Push(24.0),
            Op::Call("push_and_pop".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(24.0));
    }

    #[test]
    fn test_function_memory_isolation() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Store("x".to_string()),
            Op::Def {
                name: "store_value".to_string(),
                params: vec![],
                body: vec![
                    Op::Push(24.0),
                    Op::Store("x".to_string()),
                    Op::Return,
                ],
            },
            Op::Call("store_value".to_string()),
            Op::Load("x".to_string()),
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_nested_function_calls() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "inner".to_string(),
                params: vec![],
                body: vec![
                    Op::Push(2.0),
                    Op::Mul,
                    Op::Return,
                ],
            },
            Op::Def {
                name: "outer".to_string(),
                params: vec![],
                body: vec![
                    Op::Call("inner".to_string()),
                    Op::Push(3.0),
                    Op::Mul,
                    Op::Return,
                ],
            },
            Op::Push(7.0),
            Op::Call("outer".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0)); // 7 * 2 * 3
    }

    #[test]
    fn test_function_with_named_params() {
        let mut vm = VM::new();
        let ops = vec![
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
            Op::Push(20.0),
            Op::Push(22.0),
            Op::Call("add".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_function_missing_args() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "add".to_string(),
                params: vec!["x".to_string(), "y".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Load("y".to_string()),
                    Op::Add,
                    Op::Return,
                ],
            },
            Op::Push(42.0),
            Op::Call("add".to_string()),
        ];
         
        assert_eq!(vm.execute(&ops), Err(VMError::StackUnderflow { 
            op: "Call to function 'add'".to_string(), 
            needed: 2, 
            found: 1 
        }));
    }

    #[test]
    fn test_function_param_isolation() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "store_param".to_string(),
                params: vec!["x".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Store("x".to_string()),
                    Op::Return,
                ],
            },
            Op::Push(42.0),
            Op::Store("x".to_string()),
            Op::Push(24.0),
            Op::Call("store_param".to_string()),
            Op::Load("x".to_string()),
        ];
         
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0)); // Global x should be unchanged
    }

    #[test]
    fn test_recursive_function_with_params() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "countdown".to_string(),
                params: vec!["n".to_string()],
                body: vec![
                    Op::Load("n".to_string()),  // Load the parameter
                    Op::Push(0.0),
                    Op::Eq,   // Will push 1.0 if n==0, 0.0 otherwise
                    Op::If {
                        condition: vec![],
                        then: vec![
                            Op::Push(0.0),  // Return 0 when n==0
                        ],
                        else_: Some(vec![
                            // n > 0, so decrement and recurse
                            Op::Load("n".to_string()),
                            Op::Push(1.0),
                            Op::Sub,                       // Compute n-1
                            Op::Call("countdown".to_string()),  // Call countdown(n-1)
                        ]),
                    },
                    // Return (implicit)
                ],
            },
            Op::Push(5.0),
            Op::Call("countdown".to_string()),
        ];
         
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_nested_function_calls_with_params() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "inner".to_string(),
                params: vec!["x".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Push(2.0),
                    Op::Mul,
                    Op::Return,
                ],
            },
            Op::Def {
                name: "outer".to_string(),
                params: vec!["x".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Call("inner".to_string()),
                    Op::Push(3.0),
                    Op::Mul,
                    Op::Return,
                ],
            },
            Op::Push(7.0),
            Op::Call("outer".to_string()),
        ];
         
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0)); // 7 * 2 * 3
    }

    #[test]
    fn test_break_in_loop() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Store("counter".to_string()),
            Op::Loop {
                count: 10,
                body: vec![
                    Op::Load("counter".to_string()),
                    Op::Push(1.0),
                    Op::Add,
                    Op::Store("counter".to_string()),
                    Op::Load("counter".to_string()),
                    Op::Push(5.0),
                    Op::Eq,
                    Op::If {
                        condition: vec![],
                        then: vec![Op::Break],
                        else_: None,
                    },
                ],
            },
            Op::Load("counter".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(5.0)); // Loop should break at counter = 5
    }
    
    #[test]
    fn test_continue_in_loop() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Store("sum".to_string()),
            Op::Push(0.0),
            Op::Store("counter".to_string()),
            Op::Loop {
                count: 10,
                body: vec![
                    Op::Load("counter".to_string()),
                    Op::Push(1.0),
                    Op::Add,
                    Op::Store("counter".to_string()),
                    
                    // Skip odd numbers
                    Op::Load("counter".to_string()),
                    Op::Push(2.0),
                    Op::Mod,
                    Op::Push(0.0),
                    Op::Eq,
                    Op::Not, // If counter % 2 != 0 (odd)
                    Op::If {
                        condition: vec![],
                        then: vec![Op::Continue],
                        else_: None,
                    },
                    
                    // Add even numbers to sum
                    Op::Load("sum".to_string()),
                    Op::Load("counter".to_string()),
                    Op::Add,
                    Op::Store("sum".to_string()),
                ],
            },
            Op::Load("sum".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(30.0)); // Sum of even numbers from 2 to 10 = 2+4+6+8+10 = 30
    }
    
    #[test]
    fn test_break_in_while() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Store("counter".to_string()),
            Op::While {
                condition: vec![
                    Op::Push(1.0), // Always true
                ],
                body: vec![
                    Op::Load("counter".to_string()),
                    Op::Push(1.0),
                    Op::Add,
                    Op::Store("counter".to_string()),
                    Op::Load("counter".to_string()),
                    Op::Push(5.0),
                    Op::Eq,
                    Op::If {
                        condition: vec![],
                        then: vec![Op::Break],
                        else_: None,
                    },
                ],
            },
            Op::Load("counter".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(5.0)); // While loop should break at counter = 5
    }
    
    #[test]
    fn test_continue_in_while() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::Store("sum".to_string()),
            Op::Push(0.0),
            Op::Store("counter".to_string()),
            Op::While {
                condition: vec![
                    Op::Load("counter".to_string()),
                    Op::Push(10.0),
                    Op::Lt,
                ],
                body: vec![
                    Op::Load("counter".to_string()),
                    Op::Push(1.0),
                    Op::Add,
                    Op::Store("counter".to_string()),
                    
                    // Skip odd numbers
                    Op::Load("counter".to_string()),
                    Op::Push(2.0),
                    Op::Mod,
                    Op::Push(0.0),
                    Op::Eq,
                    Op::Not, // If counter % 2 != 0 (odd)
                    Op::If {
                        condition: vec![],
                        then: vec![Op::Continue],
                        else_: None,
                    },
                    
                    // Add even numbers to sum
                    Op::Load("sum".to_string()),
                    Op::Load("counter".to_string()),
                    Op::Add,
                    Op::Store("sum".to_string()),
                ],
            },
            Op::Load("sum".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(30.0)); // Sum of even numbers from 2 to 10 = 2+4+6+8+10 = 30
    }
    
    #[test]
    fn test_match_statement() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(2.0), // Value to match
            Op::Match {
                value: vec![],
                cases: vec![
                    (1.0, vec![Op::Push(10.0)]),
                    (2.0, vec![Op::Push(20.0)]),
                    (3.0, vec![Op::Push(30.0)]),
                ],
                default: Some(vec![Op::Push(0.0)]),
            },
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(20.0)); // Should match case 2
    }
    
    #[test]
    fn test_match_with_default() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0), // No matching case
            Op::Match {
                value: vec![],
                cases: vec![
                    (1.0, vec![Op::Push(10.0)]),
                    (2.0, vec![Op::Push(20.0)]),
                    (3.0, vec![Op::Push(30.0)]),
                ],
                default: Some(vec![Op::Push(999.0)]),
            },
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(999.0)); // Should execute default
    }
    
    #[test]
    fn test_match_no_default() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0), // No matching case
            Op::Match {
                value: vec![],
                cases: vec![
                    (1.0, vec![Op::Push(10.0)]),
                    (2.0, vec![Op::Push(20.0)]),
                    (3.0, vec![Op::Push(30.0)]),
                ],
                default: None,
            },
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(5.0)); // Should keep original value
    }
    
    #[test]
    fn test_match_with_computed_value() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Match {
                value: vec![
                    Op::Push(1.0),
                    Op::Push(2.0),
                    Op::Add,
                ],
                cases: vec![
                    (1.0, vec![Op::Push(10.0)]),
                    (3.0, vec![Op::Push(30.0)]),
                    (4.0, vec![Op::Push(40.0)]),
                ],
                default: Some(vec![Op::Push(999.0)]),
            },
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(30.0)); // 1+2=3, should match case 3
    }
    
    #[test]
    fn test_emit_event() {
        let mut vm = VM::new();
        let ops = vec![
            Op::EmitEvent {
                category: "governance".to_string(),
                message: "proposal submitted".to_string(),
            },
            Op::Push(42.0), // Just to verify execution continues
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }
    
    #[test]
    fn test_assert_equal_stack_success() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Push(42.0),
            Op::Push(42.0),
            Op::AssertEqualStack { depth: 3 },
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![42.0, 42.0, 42.0]);
    }
    
    #[test]
    fn test_assert_equal_stack_failure() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Push(24.0),
            Op::Push(42.0),
            Op::AssertEqualStack { depth: 3 },
        ];
        
        assert!(vm.execute(&ops).is_err());
    }
    
    #[test]
    fn test_assert_equal_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::AssertEqualStack { depth: 3 },
        ];
        
        assert!(vm.execute(&ops).is_err());
    }
}
