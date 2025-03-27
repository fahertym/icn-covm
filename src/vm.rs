use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Op {
    Push(f64),
    Add,
    Sub,
    Mul,
    Div,
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
}

#[derive(Debug)]
struct CallFrame {
    memory: HashMap<String, f64>,
    return_value: Option<f64>,
}

#[derive(Debug)]
pub struct VM {
    pub stack: Vec<f64>,
    memory: HashMap<String, f64>,
    functions: HashMap<String, (Vec<String>, Vec<Op>)>,
    call_frames: Vec<CallFrame>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            memory: HashMap::new(),
            functions: HashMap::new(),
            call_frames: Vec::new(),
        }
    }

    pub fn execute(&mut self, ops: &[Op]) -> Result<Option<f64>, String> {
        let mut pc = 0;
        while pc < ops.len() {
            let op = &ops[pc];
            match op {
                Op::Push(value) => {
                    self.stack.push(*value);
                }
                Op::Pop => {
                    self.stack.pop().ok_or_else(|| "Stack underflow: need a value to pop".to_string())?;
                }
                Op::Dup => {
                    let value = self.stack.last().ok_or_else(|| "Stack underflow: need a value to duplicate".to_string())?;
                    self.stack.push(*value);
                }
                Op::Swap => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values to swap".to_string());
                    }
                    let len = self.stack.len();
                    self.stack.swap(len - 1, len - 2);
                }
                Op::Over => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Over".to_string());
                    }
                    let value = self.stack[self.stack.len() - 2];
                    self.stack.push(value);
                }
                Op::Call(name) => {
                    let (params, body) = self.functions.get(name).ok_or_else(|| format!("Function '{}' not found", name))?.clone();
                    
                    let mut args = Vec::new();
                    for param in params.iter().rev() {
                        let value = self.stack.pop().ok_or_else(|| format!("Stack underflow: missing argument for parameter '{}'", param))?;
                        args.push(value);
                    }
                    args.reverse();
                    
                    let mut local_memory = HashMap::new();
                    for (param, value) in params.iter().zip(args.iter()) {
                        local_memory.insert(param.clone(), *value);
                    }
                    
                    let old_memory = std::mem::replace(&mut self.memory, local_memory);
                    self.call_frames.push(CallFrame {
                        memory: old_memory,
                        return_value: None,
                    });
                    
                    let return_result = if body.is_empty() {
                        if !args.is_empty() {
                            self.stack.push(args[0]); // Preserve first argument as return value
                        }
                        Ok(None)
                    } else {
                        self.execute(&body)
                    };
                    
                    if let Some(frame) = self.call_frames.pop() {
                        let current_memory = std::mem::replace(&mut self.memory, frame.memory);
                        // Copy non-parameter variables from current memory to frame memory
                        for (key, value) in current_memory {
                            if !params.contains(&key) {
                                self.memory.insert(key, value);
                            }
                        }
                        if let Ok(Some(retval)) = return_result {
                            self.stack.push(retval);
                        }
                    }
                }
                Op::Return => {
                    let return_value = self.stack.pop().ok_or_else(|| "Stack underflow on return".to_string())?;
                    return Ok(Some(return_value));
                }
                Op::Store(name) => {
                    let value = self.stack.pop().ok_or_else(|| "Stack underflow: need a value to store".to_string())?;
                    self.memory.insert(name.clone(), value);
                }
                Op::Load(name) => {
                    let value = self.memory.get(name).ok_or_else(|| format!("Variable '{}' not found", name))?;
                    self.stack.push(*value);
                }
                Op::If { condition, then, else_ } => {
                    self.execute(condition)?;
                    let cond = self.stack.pop().ok_or_else(|| "Stack underflow: condition block must leave a value".to_string())?;
                    
                    if cond != 0.0 {
                        self.execute(then)?;
                    } else if let Some(else_block) = else_ {
                        self.execute(else_block)?;
                    }
                }
                Op::Add => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Add".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Add".to_string())?;
                    self.stack.push(a + b);
                }
                Op::Sub => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Sub".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Sub".to_string())?;
                    self.stack.push(a - b);
                }
                Op::Mul => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Mul".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Mul".to_string())?;
                    self.stack.push(a * b);
                }
                Op::Div => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Div".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Div".to_string())?;
                    if b == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    self.stack.push(a / b);
                }
                Op::Emit(msg) => {
                    println!("{}", msg);
                }
                Op::DumpStack => {
                    println!("Stack: {:?}", self.stack);
                    println!("Final stack:");
                    for (i, value) in self.stack.iter().enumerate() {
                        println!("  {}: {}", i, value);
                    }
                }
                Op::Def { name, params, body } => {
                    self.functions.insert(name.clone(), (params.clone(), body.clone()));
                }
                Op::Loop { count, body } => {
                    for _ in 0..*count {
                        self.execute(body)?;
                    }
                }
                Op::While { condition, body } => {
                    loop {
                        self.execute(condition)?;
                        let cond = self.stack.pop().ok_or_else(|| "Stack underflow: while condition must leave a value".to_string())?;
                        
                        if cond == 0.0 {
                            break;
                        }
                        
                        self.execute(body)?;
                    }
                }
                Op::Negate => {
                    let value = self.stack.pop().ok_or_else(|| "Stack underflow: need a value to negate".to_string())?;
                    self.stack.push(-value);
                }
                Op::Not => {
                    let value = self.stack.pop().ok_or_else(|| "Stack underflow: need a value for Not".to_string())?;
                    self.stack.push(if value == 0.0 { 1.0 } else { 0.0 });
                }
                Op::And => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for And".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for And".to_string())?;
                    self.stack.push(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 });
                }
                Op::Or => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Or".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Or".to_string())?;
                    self.stack.push(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 });
                }
                Op::AssertTop(expected) => {
                    let value = self.stack.last().ok_or_else(|| "Stack underflow: need a value to assert".to_string())?;
                    if *value != *expected {
                        return Err(format!("Assertion failed: expected {}, got {}", expected, value));
                    }
                }
                Op::DumpMemory => {
                    println!("Memory contents:");
                    for (key, value) in self.memory.iter() {
                        println!("  {} = {}", key, value);
                    }
                }
                Op::AssertMemory { key, expected } => {
                    let value = self.memory.get(key).ok_or_else(|| format!("Variable '{}' not found", key))?;
                    if *value != *expected {
                        return Err(format!("Assertion failed: expected {} = {}, got {}", key, expected, value));
                    }
                }
                Op::Lt => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Lt".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Lt".to_string())?;
                    self.stack.push(if a < b { 1.0 } else { 0.0 });
                }
                Op::Gt => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Gt".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Gt".to_string())?;
                    self.stack.push(if a > b { 1.0 } else { 0.0 });
                }
                Op::Eq => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Eq".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow: need at least 2 values for Eq".to_string())?;
                    self.stack.push(if a == b { 1.0 } else { 0.0 });
                }
            }
            pc += 1;
        }
        Ok(None)
    }

    // These methods are used in tests
    #[cfg(test)]
    pub fn top(&self) -> Option<f64> {
        self.stack.last().copied()
    }

    #[cfg(test)]
    pub fn get_memory(&self, key: &str) -> Option<f64> {
        self.memory.get(key).copied()
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

        assert_eq!(vm.execute(&ops), Err("Division by zero".to_string()));
    }

    #[test]
    fn test_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(5.0), Op::Add];

        assert_eq!(
            vm.execute(&ops),
            Err("Stack underflow: need at least 2 values for Add".to_string())
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

        assert_eq!(vm.execute(&ops), Err("Variable 'nonexistent' not found".to_string()));
    }

    #[test]
    fn test_store_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Store("x".to_string())];

        assert_eq!(
            vm.execute(&ops),
            Err("Stack underflow: need a value to store".to_string())
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
            Op::Push(1.0),
            Op::If {
                condition: vec![Op::Push(1.0)],
                then: vec![Op::Push(42.0)],
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_if_zero_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::If {
                condition: vec![Op::Push(0.0)],
                then: vec![Op::Push(42.0)],
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0));
    }

    #[test]
    fn test_if_zero_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::If {
            condition: vec![],
            then: vec![Op::Push(42.0)],
            else_: None,
        }];

        assert_eq!(vm.execute(&ops), Err("Stack underflow: condition block must leave a value".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value to negate".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Assertion failed: expected 24, got 42".to_string()));
    }

    #[test]
    fn test_assert_top_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::AssertTop(42.0)];
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value to assert".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value for Not".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values for And".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values for Or".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: while condition must leave a value".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value to duplicate".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values to swap".to_string()));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values for Over".to_string()));
    }

    #[test]
    fn test_stack_manipulation_chain() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0),
            Op::Push(2.0),
            Op::Push(3.0),
            Op::Dup,    // Stack: [1.0, 2.0, 3.0, 3.0]
            Op::Swap,   // Stack: [1.0, 2.0, 3.0, 3.0]
            Op::Over,   // Stack: [1.0, 2.0, 3.0, 3.0, 3.0]
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
        
        assert_eq!(vm.execute(&ops), Err("Function 'nonexistent' not found".to_string()));
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
                params: vec!["n".to_string()],
                body: vec![
                    Op::Load("n".to_string()),
                    Op::Push(0.0),
                    Op::Eq,
                    Op::If {
                        condition: vec![Op::Push(1.0)],
                        then: vec![
                            Op::Push(0.0),
                            Op::Return,
                        ],
                        else_: Some(vec![
                            Op::Load("n".to_string()),
                            Op::Push(1.0),
                            Op::Sub,
                            Op::Call("countdown".to_string()),
                            Op::Return,
                        ]),
                    },
                ],
            },
            Op::Push(3.0),
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
            Op::Store("global".to_string()),
            Op::Def {
                name: "store_value".to_string(),
                params: vec!["x".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Store("local".to_string()),
                    Op::Push(0.0),
                    Op::Return,
                ],
            },
            Op::Push(24.0),
            Op::Call("store_value".to_string()),
            Op::Load("global".to_string()),
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
            Op::Call("add".to_string()),
        ];
         
        assert_eq!(vm.execute(&ops), Err("Stack underflow: missing argument for parameter 'y'".to_string()));
    }

    #[test]
    fn test_function_param_isolation() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(42.0),
            Op::Store("x".to_string()),
            Op::Def {
                name: "store_param".to_string(),
                params: vec!["x".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Store("x".to_string()),
                    Op::Push(0.0),
                    Op::Return,
                ],
            },
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
                    Op::Load("n".to_string()),
                    Op::Push(0.0),
                    Op::Eq,
                    Op::If {
                        condition: vec![Op::Push(1.0)],
                        then: vec![
                            Op::Push(0.0),
                            Op::Return,
                        ],
                        else_: Some(vec![
                            Op::Load("n".to_string()),
                            Op::Push(1.0),
                            Op::Sub,
                            Op::Call("countdown".to_string()),
                            Op::Return,
                        ]),
                    },
                ],
            },
            Op::Push(4.0),
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
    fn test_function_with_side_effects() {
        let mut vm = VM::new();
        let ops = vec![
            // Set up global state
            Op::Push(42.0),
            Op::Store("global".to_string()),
            Op::Push(100.0),
            Op::Store("shared".to_string()),
            
            // Define function that modifies shared state
            Op::Def {
                name: "modify_shared".to_string(),
                params: vec!["x".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Store("shared".to_string()),
                    Op::Push(0.0),
                    Op::Return,
                ],
            },
            
            // Call function and verify global state
            Op::Push(200.0),
            Op::Call("modify_shared".to_string()),
            Op::Load("global".to_string()),
            Op::Load("shared".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.get_memory("global"), Some(42.0)); // Global should be unchanged
        assert_eq!(vm.get_memory("shared"), Some(200.0)); // Shared should be modified
    }

    #[test]
    fn test_function_returns_stack_value() {
        let mut vm = VM::new();
        let ops = vec![
            // Define function that doesn't explicitly return
            Op::Def {
                name: "no_return".to_string(),
                params: vec!["x".to_string()],
                body: vec![
                    Op::Load("x".to_string()),
                    Op::Push(2.0),
                    Op::Mul,
                ],
            },
            
            // Call function and verify stack state
            Op::Push(21.0),
            Op::Call("no_return".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0)); // Should have computed value but not returned
    }

    #[test]
    fn test_function_empty_body() {
        let mut vm = VM::new();
        let ops = vec![
            // Define function with empty body
            Op::Def {
                name: "empty".to_string(),
                params: vec!["x".to_string()],
                body: vec![],
            },
            
            // Call function and verify stack state
            Op::Push(42.0),
            Op::Call("empty".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0)); // Stack should be unchanged
    }

    #[test]
    fn test_nested_recursive_calls() {
        let mut vm = VM::new();
        let ops = vec![
            // Define inner recursive function
            Op::Def {
                name: "inner".to_string(),
                params: vec!["n".to_string()],
                body: vec![
                    Op::Load("n".to_string()),
                    Op::Push(0.0),
                    Op::Eq,
                    Op::If {
                        condition: vec![Op::Push(1.0)],
                        then: vec![
                            Op::Push(0.0),
                            Op::Return,
                        ],
                        else_: Some(vec![
                            Op::Load("n".to_string()),
                            Op::Push(1.0),
                            Op::Sub,
                            Op::Call("inner".to_string()),
                            Op::Return,
                        ]),
                    },
                ],
            },
            
            // Define outer function that calls inner
            Op::Def {
                name: "outer".to_string(),
                params: vec!["n".to_string()],
                body: vec![
                    Op::Load("n".to_string()),
                    Op::Call("inner".to_string()),
                    Op::Return,
                ],
            },
            
            // Test nested recursion
            Op::Push(3.0),
            Op::Call("outer".to_string()),
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(0.0)); // Should count down to 0
    }
}
