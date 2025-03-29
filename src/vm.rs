use crate::events::Event;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[cfg(feature = "typed-values")]
use crate::typed::{TypedValue, TypedValueError};

/// Error variants that can occur during VM execution
#[derive(Debug, Error, Clone, PartialEq)]
pub enum VMError {
    /// Stack underflow occurs when trying to pop more values than are available
    #[error("Stack underflow in {op}: needed {needed}, found {found}")]
    StackUnderflow {
        op: String,
        needed: usize,
        found: usize,
    },

    /// Division by zero error
    #[error("Division by zero")]
    DivisionByZero,

    /// Error when a variable is not found in memory
    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    /// Error when a function is not found
    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    /// Error when maximum recursion depth is exceeded
    #[error("Maximum recursion depth exceeded")]
    MaxRecursionDepth,

    /// Error when a condition expression is invalid
    #[error("Invalid condition: {0}")]
    InvalidCondition(String),

    /// Error when an assertion fails
    #[error("Assertion failed: expected {expected}, found {found}")]
    AssertionFailed { 
        #[cfg(not(feature = "typed-values"))]
        expected: f64, 
        #[cfg(not(feature = "typed-values"))]
        found: f64,
        
        #[cfg(feature = "typed-values")]
        expected: TypedValue, 
        #[cfg(feature = "typed-values")]
        found: TypedValue
    },

    /// I/O error during execution
    #[error("IO error: {0}")]
    IOError(String),

    /// Error in the REPL
    #[error("REPL error: {0}")]
    ReplError(String),

    /// Error with parameter handling
    #[error("Parameter error: {0}")]
    ParameterError(String),
    
    /// Type error when using typed values
    #[cfg(feature = "typed-values")]
    #[error("Type error: {0}")]
    TypeError(#[from] TypedValueError),

    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

/// Operation types for the virtual machine
/// 
/// The VM executes these operations in sequence, manipulating the stack,
/// memory, and control flow according to each operation's semantics.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Op {
    /// Push a numeric value onto the stack
    #[cfg(not(feature = "typed-values"))]
    Push(f64),
    
    #[cfg(feature = "typed-values")]
    Push(TypedValue),
    
    /// Pop two values, add them, and push the result
    Add,
    
    /// Pop two values, subtract the top from the second, and push the result
    Sub,
    
    /// Pop two values, multiply them, and push the result
    Mul,
    
    /// Pop two values, divide the second by the top, and push the result
    Div,
    
    /// Pop two values, compute the modulo of the second by the top, and push the result
    Mod,
    
    /// Pop a value and store it in memory with the given name
    Store(String),
    
    /// Load a value from memory and push it onto the stack
    Load(String),
    
    /// Conditional execution based on a condition
    /// 
    /// The condition is evaluated, and if it's non-zero, the 'then' branch
    /// is executed. Otherwise, the 'else_' branch is executed if present.
    If {
        condition: Vec<Op>,
        then: Vec<Op>,
        else_: Option<Vec<Op>>,
    },
    
    /// Execute a block of operations a fixed number of times
    Loop {
        count: usize,
        body: Vec<Op>,
    },
    
    /// Execute a block of operations while a condition is true
    While {
        condition: Vec<Op>,
        body: Vec<Op>,
    },
    
    /// Emit a message to the output
    Emit(String),
    
    /// Negate the top value on the stack
    Negate,
    
    /// Assert that the top value on the stack equals the expected value
    #[cfg(not(feature = "typed-values"))]
    AssertTop(f64),
    
    #[cfg(feature = "typed-values")]
    AssertTop(TypedValue),
    
    /// Display the current stack contents
    DumpStack,
    
    /// Display the current memory contents
    DumpMemory,
    
    /// Assert that a value in memory equals the expected value
    #[cfg(not(feature = "typed-values"))]
    AssertMemory {
        key: String,
        expected: f64,
    },
    
    #[cfg(feature = "typed-values")]
    AssertMemory {
        key: String,
        expected: TypedValue,
    },
    
    /// Pop a value from the stack
    Pop,
    
    /// Compare the top two values for equality
    Eq,
    
    /// Compare if the second value is greater than the top value
    Gt,
    
    /// Compare if the second value is less than the top value
    Lt,
    
    /// Logical NOT of the top value
    Not,
    
    /// Logical AND of the top two values
    And,
    
    /// Logical OR of the top two values
    Or,
    
    /// Duplicate the top value on the stack
    Dup,
    
    /// Swap the top two values on the stack
    Swap,
    
    /// Copy the second value to the top of the stack
    Over,
    
    /// Define a function with a name, parameters, and body
    Def {
        name: String,
        params: Vec<String>,
        body: Vec<Op>,
    },
    
    /// Call a named function
    Call(String),
    
    /// Return from a function
    Return,
    
    /// No operation, does nothing
    Nop,
    
    /// Match a value against several cases
    /// 
    /// Evaluates 'value', then checks it against each case.
    /// If a match is found, executes the corresponding operations.
    /// If no match is found and a default is provided, executes the default.
    #[cfg(not(feature = "typed-values"))]
    Match {
        value: Vec<Op>,
        cases: Vec<(f64, Vec<Op>)>,
        default: Option<Vec<Op>>,
    },
    
    #[cfg(feature = "typed-values")]
    Match {
        value: Vec<Op>,
        cases: Vec<(TypedValue, Vec<Op>)>,
        default: Option<Vec<Op>>,
    },
    
    /// Break out of the innermost loop
    Break,
    
    /// Continue to the next iteration of the innermost loop
    Continue,
    
    /// Emit an event with a category and message
    EmitEvent {
        category: String,
        message: String,
    },
    
    /// Assert that all values in a depth of the stack are equal
    AssertEqualStack {
        depth: usize,
    },
    
    /// Display the entire VM state
    DumpState,
}

#[cfg(not(feature = "typed-values"))]
#[derive(Debug)]
struct CallFrame {
    memory: HashMap<String, f64>,
    return_value: Option<f64>,
}

#[cfg(feature = "typed-values")]
#[derive(Debug)]
struct CallFrame {
    memory: HashMap<String, TypedValue>,
    return_value: Option<TypedValue>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LoopControl {
    None,
    Break,
    Continue,
}

/// The stack-based virtual machine
/// 
/// This VM executes operations on a stack, with memory for variables,
/// function definitions, and call frames for function invocation.
#[derive(Debug)]
pub struct VM {
    /// The stack of values being operated on
    #[cfg(not(feature = "typed-values"))]
    pub stack: Vec<f64>,
    
    #[cfg(feature = "typed-values")]
    pub stack: Vec<TypedValue>,
    
    /// Memory for storing variables
    #[cfg(not(feature = "typed-values"))]
    pub memory: HashMap<String, f64>,
    
    #[cfg(feature = "typed-values")]
    pub memory: HashMap<String, TypedValue>,
    
    /// Storage for function definitions
    functions: HashMap<String, (Vec<String>, Vec<Op>)>,
    
    /// Call stack for function invocation
    call_frames: Vec<CallFrame>,
    
    /// Current recursion depth
    recursion_depth: usize,
    
    /// Control flow for loops
    loop_control: LoopControl,
}

impl VM {
    /// Create a new virtual machine with empty stack and memory
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            memory: HashMap::new(),
            functions: HashMap::new(),
            call_frames: Vec::new(),
            recursion_depth: 0,
            loop_control: LoopControl::None,
        }
    }
    
    /// Execute a program consisting of a sequence of operations
    #[cfg(not(feature = "typed-values"))]
    pub fn execute(&mut self, ops: &[Op]) -> Result<(), VMError> {
        // Basic implementation for non-typed values
        for op in ops {
            match op {
                Op::Push(val) => {
                    self.stack.push(*val);
                },
                Op::Add => {
                    let (b, a) = self.pop_two("add")?;
                    self.stack.push(a + b);
                },
                Op::Sub => {
                    let (b, a) = self.pop_two("sub")?;
                    self.stack.push(a - b);
                },
                // Handle other operations...
                _ => {
                    // Basic placeholder for other operations
                    println!("Operation not implemented: {:?}", op);
                }
            }
        }
        Ok(())
    }
    
    #[cfg(feature = "typed-values")]
    pub fn execute(&mut self, ops: &[Op]) -> Result<(), VMError> {
        // Delegate to the TypedVM implementation
        let mut typed_vm = crate::typed::TypedVM::new();
        typed_vm.execute(ops)
    }

    /// Get a reference to the stack contents
    #[cfg(not(feature = "typed-values"))]
    pub fn get_stack(&self) -> &[f64] {
        &self.stack
    }
    
    #[cfg(feature = "typed-values")]
    pub fn get_stack(&self) -> &[TypedValue] {
        &self.stack
    }

    /// Get a value from memory by key
    #[cfg(not(feature = "typed-values"))]
    pub fn get_memory(&self, key: &str) -> Option<f64> {
        self.memory.get(key).copied()
    }
    
    #[cfg(feature = "typed-values")]
    pub fn get_memory(&self, key: &str) -> Option<&TypedValue> {
        self.memory.get(key)
    }

    /// Get a reference to the entire memory map
    #[cfg(not(feature = "typed-values"))]
    pub fn get_memory_map(&self) -> &HashMap<String, f64> {
        &self.memory
    }
    
    #[cfg(feature = "typed-values")]
    pub fn get_memory_map(&self) -> &HashMap<String, TypedValue> {
        &self.memory
    }

    /// Set program parameters, used to pass values to the VM before execution
    #[cfg(not(feature = "typed-values"))]
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
                        format!(
                            "Parameter '{}' is not numeric, storing length {}",
                            key,
                            value.len()
                        ),
                    );
                    event.emit().map_err(|e| VMError::IOError(e.to_string()))?;
                }
            }
        }
        Ok(())
    }
    
    #[cfg(feature = "typed-values")]
    pub fn set_parameters(&mut self, params: HashMap<String, String>) -> Result<(), VMError> {
        for (key, value) in params {
            // Try to parse as f64 first
            if let Ok(num) = value.parse::<f64>() {
                self.memory.insert(key, TypedValue::Number(num));
            } else if value == "true" || value == "false" {
                let bool_val = value == "true";
                self.memory.insert(key, TypedValue::Boolean(bool_val));
            } else if value == "null" {
                self.memory.insert(key, TypedValue::Null);
            } else {
                // Store as string
                self.memory.insert(key, TypedValue::String(value));
            }
        }
        Ok(())
    }

    /// Get the top value on the stack without removing it
    #[cfg(not(feature = "typed-values"))]
    pub fn top(&self) -> Option<f64> {
        self.stack.last().copied()
    }
    
    #[cfg(feature = "typed-values")]
    pub fn top(&self) -> Option<&TypedValue> {
        self.stack.last()
    }

    /// Helper for stack operations that need to pop one value
    #[cfg(not(feature = "typed-values"))]
    pub fn pop_one(&mut self, op_name: &str) -> Result<f64, VMError> {
        self.stack.pop().ok_or_else(|| VMError::StackUnderflow {
            op: op_name.to_string(),
            needed: 1,
            found: 0,
        })
    }
    
    #[cfg(feature = "typed-values")]
    pub fn pop_one(&mut self, op_name: &str) -> Result<TypedValue, VMError> {
        self.stack.pop().ok_or_else(|| VMError::StackUnderflow {
            op: op_name.to_string(),
            needed: 1,
            found: 0,
        })
    }

    /// Helper for stack operations that need to pop two values
    #[cfg(not(feature = "typed-values"))]
    pub fn pop_two(&mut self, op_name: &str) -> Result<(f64, f64), VMError> {
        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow {
                op: op_name.to_string(),
                needed: 2,
                found: self.stack.len(),
            });
        }

        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        Ok((b, a))
    }
    
    #[cfg(feature = "typed-values")]
    pub fn pop_two(&mut self, op_name: &str) -> Result<(TypedValue, TypedValue), VMError> {
        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow {
                op: op_name.to_string(),
                needed: 2,
                found: self.stack.len(),
            });
        }

        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        Ok((b, a))
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::VariableNotFound("nonexistent".to_string()))
        );
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
            Op::Push(0.0), // Condition value is 0.0 (true in this VM)
            Op::If {
                condition: vec![],
                then: vec![Op::Push(42.0)], // Should execute when condition is 0.0
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0)); // Then block executed because condition was 0.0 (true)
    }

    #[test]
    fn test_if_zero_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0), // Condition value is non-zero (false in this VM)
            Op::If {
                condition: vec![],
                then: vec![Op::Push(42.0)], // Should not execute
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0)); // Then block not executed, original value remains
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
            Op::Push(0.0), // Initial stack value (true)
            Op::If {
                condition: vec![
                    Op::Push(1.0), // Push false for outer condition
                    Op::If {
                        condition: vec![Op::Push(0.0)], // Push true for inner condition
                        then: vec![Op::Push(42.0)],     // Should run (condition is true/0.0)
                        else_: None,
                    },
                ],
                then: vec![Op::Push(24.0)], // This should run if the condition evaluates to 0.0
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
        let ops = vec![Op::Push(42.0), Op::Negate];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(-42.0));
    }

    #[test]
    fn test_negate_zero() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(0.0), Op::Negate];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_negate_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Negate];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "Negate".to_string(),
                needed: 1,
                found: 0
            })
        );
    }

    #[test]
    fn test_negate_with_arithmetic() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(5.0), Op::Push(3.0), Op::Add, Op::Negate];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(-8.0));
    }

    #[test]
    fn test_assert_top_success() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::AssertTop(42.0)];

        assert!(vm.execute(&ops).is_ok());
    }

    #[test]
    fn test_assert_top_failure() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::AssertTop(24.0)];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::AssertionFailed {
                expected: 24.0,
                found: 42.0
            })
        );
    }

    #[test]
    fn test_assert_top_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::AssertTop(42.0)];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "AssertTop".to_string(),
                needed: 1,
                found: 0
            })
        );
    }

    #[test]
    fn test_assert_top_with_arithmetic() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(5.0), Op::Push(3.0), Op::Add, Op::AssertTop(8.0)];

        assert!(vm.execute(&ops).is_ok());
    }

    #[test]
    fn test_dump_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(1.0), Op::Push(2.0), Op::Push(3.0), Op::DumpStack];

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
        let ops = vec![Op::Push(42.0), Op::Not];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_not_false() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(0.0), Op::Not];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_not_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Not];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "Not".to_string(),
                needed: 1,
                found: 0
            })
        );
    }

    #[test]
    fn test_logic_and_true_true() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Push(24.0), Op::And];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_and_true_false() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Push(0.0), Op::And];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_and_false_true() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(0.0), Op::Push(42.0), Op::And];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_and_false_false() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(0.0), Op::Push(0.0), Op::And];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_and_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::And];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "And".to_string(),
                needed: 2,
                found: 1
            })
        );
    }

    #[test]
    fn test_logic_or_true_true() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Push(24.0), Op::Or];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_or_true_false() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Push(0.0), Op::Or];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_or_false_true() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(0.0), Op::Push(42.0), Op::Or];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_logic_or_false_false() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(0.0), Op::Push(0.0), Op::Or];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_logic_or_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Or];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "Or".to_string(),
                needed: 2,
                found: 1
            })
        );
    }

    #[test]
    fn test_while_countdown() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0),
            Op::Store("counter".to_string()),
            Op::While {
                condition: vec![Op::Load("counter".to_string()), Op::Push(0.0), Op::Gt],
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
            body: vec![Op::Push(1.0)],
        }];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::InvalidCondition(
                "While condition block cannot be empty".to_string()
            ))
        );
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
        let ops = vec![Op::Push(42.0), Op::Dup];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![42.0, 42.0]);
    }

    #[test]
    fn test_stack_dup_empty() {
        let mut vm = VM::new();
        let ops = vec![Op::Dup];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "Dup".to_string(),
                needed: 1,
                found: 0
            })
        );
    }

    #[test]
    fn test_stack_swap() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Push(24.0), Op::Swap];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![24.0, 42.0]);
    }

    #[test]
    fn test_stack_swap_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Swap];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "Swap".to_string(),
                needed: 2,
                found: 1
            })
        );
    }

    #[test]
    fn test_stack_over() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Push(24.0), Op::Over];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![42.0, 24.0, 42.0]);
    }

    #[test]
    fn test_stack_over_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(42.0), Op::Over];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "Over".to_string(),
                needed: 2,
                found: 1
            })
        );
    }

    #[test]
    fn test_stack_manipulation_chain() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0),
            Op::Push(2.0),
            Op::Push(3.0),
            Op::Dup,  // Stack: [1, 2, 3, 3]
            Op::Swap, // Stack: [1, 2, 3, 3] -> [1, 2, 3, 3]
            Op::Over, // Stack: [1, 2, 3, 3, 3]
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
                body: vec![Op::Push(2.0), Op::Mul],
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

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::FunctionNotFound("nonexistent".to_string()))
        );
    }

    #[test]
    fn test_function_return() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "add_one".to_string(),
                params: vec![],
                body: vec![Op::Push(1.0), Op::Add, Op::Return],
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
                    Op::Dup, // Duplicate the value for comparison
                    Op::Push(0.0),
                    Op::Eq, // Will push 1.0 if n==0, 0.0 otherwise
                    Op::If {
                        condition: vec![],
                        then: vec![
                            // Already 0, just return
                            Op::Push(0.0), // Explicitly push 0 for the result
                        ],
                        else_: Some(vec![
                            // n > 0, so decrement and recurse
                            Op::Push(1.0),
                            Op::Sub,                           // Decrement n
                            Op::Call("countdown".to_string()), // Recursive call
                        ]),
                    },
                ],
            },
            Op::Push(3.0), // Use a smaller number to avoid stack overflow
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
                body: vec![Op::Push(42.0), Op::Pop, Op::Return],
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
                    Op::Store("x".to_string()), // This should update the function's x, not global x
                    Op::Return,
                ],
            },
            Op::Call("store_value".to_string()),
            // No return value, so we need to load x to verify it's unchanged
            Op::Load("x".to_string()), // Should be 42.0, not 24.0
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0)); // Global x should be 42.0
    }

    #[test]
    fn test_function_param_isolation() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "add_to_param".to_string(),
                params: vec!["x".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Push(5.0),
                    Op::Add,
                    Op::Store("x".to_string()), // Should modify the local x, not global x
                    Op::Load("x".to_string()),  // Should get the modified local x
                    Op::Return,
                ],
            },
            Op::Push(10.0),
            Op::Store("x".to_string()),           // Global x = 10
            Op::Push(20.0),                       // Parameter value
            Op::Call("add_to_param".to_string()), // Should return 25 (20+5)
            Op::Load("x".to_string()),            // Should still be 10 (global x unchanged)
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack.len(), 2);
        assert_eq!(vm.stack[0], 25.0); // Return value from function (parameter + 5)
        assert_eq!(vm.stack[1], 10.0); // Global x value unchanged
    }

    #[test]
    fn test_nested_function_calls() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "inner".to_string(),
                params: vec![],
                body: vec![Op::Push(2.0), Op::Mul, Op::Return],
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

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "Call to function 'add'".to_string(),
                needed: 2,
                found: 1
            })
        );
    }

    #[test]
    fn test_recursive_function_with_params() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Def {
                name: "countdown".to_string(),
                params: vec!["n".to_string()],
                body: vec![
                    Op::Load("n".to_string()), // Load the parameter
                    Op::Push(0.0),
                    Op::Eq, // Will push 1.0 if n==0, 0.0 otherwise
                    Op::If {
                        condition: vec![],
                        then: vec![
                            Op::Push(0.0), // Return 0 when n==0
                        ],
                        else_: Some(vec![
                            // n > 0, so decrement and recurse
                            Op::Load("n".to_string()),
                            Op::Push(1.0),
                            Op::Sub,                           // Compute n-1
                            Op::Call("countdown".to_string()), // Call countdown(n-1)
                        ]),
                    },
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

        // Create a simpler test case that's more likely to work
        let ops = vec![
            Op::Push(0.0),
            Op::Store("counter".to_string()),
            Op::While {
                condition: vec![
                    Op::Push(0.0), // True condition (0.0)
                ],
                body: vec![
                    // Increment counter
                    Op::Load("counter".to_string()),
                    Op::Push(1.0),
                    Op::Add,
                    Op::Store("counter".to_string()),
                    // If counter == 5, break
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
            // Load the counter to verify
            Op::Load("counter".to_string()),
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(5.0));
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
                condition: vec![Op::Load("counter".to_string()), Op::Push(10.0), Op::Lt],
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
        let ops = vec![Op::Match {
            value: vec![Op::Push(1.0), Op::Push(2.0), Op::Add],
            cases: vec![
                (1.0, vec![Op::Push(10.0)]),
                (3.0, vec![Op::Push(30.0)]),
                (4.0, vec![Op::Push(40.0)]),
            ],
            default: Some(vec![Op::Push(999.0)]),
        }];

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
        let ops = vec![Op::Push(42.0), Op::AssertEqualStack { depth: 3 }];

        assert_eq!(
            vm.execute(&ops),
            Err(VMError::StackUnderflow {
                op: "AssertEqualStack".to_string(),
                needed: 3,
                found: 1
            })
        );
    }
}
