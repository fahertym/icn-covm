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
    If { then: Vec<Op>, else_: Option<Vec<Op>> },
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
    DumpCallStack,
    DumpFunctions,
    DumpStackWithLabels { labels: Vec<String> },
    DumpMemoryWithLabels { labels: HashMap<String, String> },
    DebugBreak { message: Option<String> },
    Trace { enabled: bool },
}

#[derive(Debug)]
struct CallFrame {
    memory: HashMap<String, f64>,
    return_value: Option<f64>,
    function_name: String,
    params: Vec<String>,
}

#[derive(Debug)]
pub struct VM {
    pub stack: Vec<f64>,
    memory: HashMap<String, f64>,
    functions: HashMap<String, (Vec<String>, Vec<Op>)>,
    call_frames: Vec<CallFrame>,
    tracing_enabled: bool,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            memory: HashMap::new(),
            functions: HashMap::new(),
            call_frames: Vec::new(),
            tracing_enabled: false,
        }
    }

    pub fn execute(&mut self, ops: &[Op]) -> Result<(), String> {
        let mut pc = 0;
        while pc < ops.len() {
            let op = &ops[pc];
            
            if self.tracing_enabled {
                println!("Executing op at {}: {:?}", pc, op);
            }

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
                Op::Call(name) => {
                    let (params, body) = self.functions.get(name).ok_or_else(|| format!("Function '{}' not found", name))?.clone();
                    
                    let mut local_memory = HashMap::new();
                    
                    for param in params.iter().rev() {
                        let value = self.stack.pop().ok_or_else(|| format!("Stack underflow: missing argument for parameter '{}'", param))?;
                        local_memory.insert(param.clone(), value);
                    }
                    
                    self.call_frames.push(CallFrame {
                        memory: std::mem::replace(&mut self.memory, local_memory),
                        return_value: None,
                        function_name: name.clone(),
                        params: params.clone(),
                    });
                    
                    self.execute(&body)?;
                    
                    if let Some(frame) = self.call_frames.pop() {
                        self.memory = frame.memory;
                        if let Some(return_value) = frame.return_value {
                            self.stack.push(return_value);
                        }
                    }
                }
                Op::Return => {
                    let return_value = self.stack.pop();
                    
                    if let Some(frame) = self.call_frames.last_mut() {
                        frame.return_value = return_value;
                    }
                    
                    return Ok(());
                }
                Op::Load(name) => {
                    let value = self.memory.get(name).ok_or_else(|| format!("Variable '{}' not found", name))?;
                    self.stack.push(*value);
                }
                Op::Store(name) => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value to store".to_string());
                    }
                    let value = self.stack.pop().unwrap();
                    self.memory.insert(name.clone(), value);
                }
                Op::Add => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Add".to_string());
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a + b);
                }
                Op::Sub => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Sub".to_string());
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a - b);
                }
                Op::Mul => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Mul".to_string());
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a * b);
                }
                Op::Div => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Div".to_string());
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    if b == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    self.stack.push(a / b);
                }
                Op::Lt => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow".to_string())?;
                    self.stack.push(if a < b { 1.0 } else { 0.0 });
                }
                Op::Gt => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow".to_string())?;
                    self.stack.push(if a > b { 1.0 } else { 0.0 });
                }
                Op::Eq => {
                    let b = self.stack.pop().ok_or_else(|| "Stack underflow".to_string())?;
                    let a = self.stack.pop().ok_or_else(|| "Stack underflow".to_string())?;
                    self.stack.push(if a == b { 1.0 } else { 0.0 });
                }
                Op::If { then, else_ } => {
                    let cond = self.stack.pop().ok_or_else(|| "Stack underflow: need a value for If".to_string())?;
                    if cond != 0.0 {
                        self.execute(then)?;
                    } else if let Some(else_block) = else_ {
                        self.execute(else_block)?;
                    }
                }
                Op::Loop { count, body } => {
                    for _ in 0..*count {
                        self.execute(body)?;
                    }
                }
                Op::Negate => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value to negate".to_string());
                    }
                    let value = self.stack.pop().unwrap();
                    self.stack.push(-value);
                }
                Op::Not => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value for Not".to_string());
                    }
                    let value = self.stack.pop().unwrap();
                    self.stack.push(if value == 0.0 { 1.0 } else { 0.0 });
                }
                Op::And => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for And".to_string());
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 });
                }
                Op::Or => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Or".to_string());
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 });
                }
                Op::While { condition, body } => {
                    loop {
                        self.execute(condition)?;
                        let cond = self.stack.pop().ok_or_else(|| "Stack underflow in while condition".to_string())?;
                        if cond == 0.0 {
                            break;
                        }
                        self.execute(body)?;
                    }
                }
                Op::AssertTop(expected) => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value to assert".to_string());
                    }
                    let value = self.stack.last().unwrap();
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
                Op::DumpCallStack => {
                    println!("Call Stack:");
                    for (i, frame) in self.call_frames.iter().enumerate() {
                        println!("  Frame {}: {} with params {:?}", i, frame.function_name, frame.params);
                    }
                }
                Op::DumpFunctions => {
                    println!("Defined Functions:");
                    for (name, (params, _)) in self.functions.iter() {
                        println!("  {} with params {:?}", name, params);
                    }
                }
                Op::DumpStackWithLabels { labels } => {
                    println!("Stack with labels:");
                    for (i, value) in self.stack.iter().enumerate() {
                        let label = labels.get(i).map(|s| s.as_str()).unwrap_or("unnamed");
                        println!("  {}: {} = {}", i, label, value);
                    }
                }
                Op::DumpMemoryWithLabels { labels } => {
                    println!("Memory with labels:");
                    for (key, value) in self.memory.iter() {
                        let label = labels.get(key).map(|s| s.as_str()).unwrap_or("unnamed");
                        println!("  {} ({}) = {}", key, label, value);
                    }
                }
                Op::DebugBreak { message } => {
                    if let Some(msg) = message {
                        println!("Debug break: {}", msg);
                    } else {
                        println!("Debug break");
                    }
                }
                Op::Trace { enabled } => {
                    self.tracing_enabled = *enabled;
                    println!("Instruction tracing {}", if *enabled { "enabled" } else { "disabled" });
                }
            }
            pc += 1;
        }
        Ok(())
    }

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
            Op::Push(1.0), // condition is true
            Op::If {
                then: vec![Op::Push(42.0)],
                else_: Some(vec![Op::Push(0.0)]),
            },
        ];
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_if_zero_false() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0), // condition is false
            Op::If {
                then: vec![Op::Push(42.0)],
                else_: Some(vec![Op::Push(1.0)]),
            },
        ];
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(1.0));
    }

    #[test]
    fn test_if_zero_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![
            Op::If {
                then: vec![Op::Push(42.0)],
                else_: Some(vec![Op::Push(0.0)]),
            },
        ];
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value for If".to_string()));
    }

    #[test]
    fn test_nested_if_zero() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0),
            Op::If {
                then: vec![
                    Op::Push(0.0),
                    Op::If {
                        then: vec![Op::Push(42.0)],
                        else_: Some(vec![Op::Push(24.0)]),
                    },
                ],
                else_: Some(vec![Op::Push(0.0)]),
            },
        ];
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(24.0));
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
        assert_eq!(vm.top(), Some(16.0));
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
    fn test_logic_not_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Not];
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value for Not".to_string()));
    }
}
