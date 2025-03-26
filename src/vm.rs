use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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
}

#[derive(Debug)]
pub struct VM {
    pub stack: Vec<f64>,
    memory: std::collections::HashMap<String, f64>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            memory: std::collections::HashMap::new(),
        }
    }

    pub fn execute(&mut self, ops: &[Op]) -> Result<(), &'static str> {
        for op in ops {
            match op {
                Op::Push(value) => {
                    self.stack.push(*value);
                }
                Op::Add => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Add");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a + b);
                }
                Op::Sub => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Sub");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a - b);
                }
                Op::Mul => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Mul");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a * b);
                }
                Op::Div => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Div");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    if b == 0.0 {
                        return Err("Division by zero");
                    }
                    self.stack.push(a / b);
                }
                Op::Store(key) => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value to store");
                    }
                    let value = self.stack.pop().unwrap();
                    self.memory.insert(key.clone(), value);
                }
                Op::Load(key) => {
                    if let Some(&value) = self.memory.get(key) {
                        self.stack.push(value);
                    } else {
                        return Err("Key not found in memory");
                    }
                }
                Op::If { condition, then, else_ } => {
                    // Execute condition block
                    self.execute(condition)?;
                    
                    // Check result
                    if self.stack.is_empty() {
                        return Err("Stack underflow: condition block must leave a value");
                    }
                    let result = self.stack.pop().unwrap();
                    
                    // Execute appropriate branch
                    if result != 0.0 {
                        self.execute(then)?;
                    } else if let Some(else_block) = else_ {
                        self.execute(&else_block)?;
                    }
                }
                Op::Loop { count, body } => {
                    for _ in 0..*count {
                        self.execute(body)?;
                    }
                }
                Op::Emit(message) => {
                    println!("{}", message);
                }
                Op::Negate => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value to negate");
                    }
                    let value = self.stack.pop().unwrap();
                    self.stack.push(-value);
                }
                Op::AssertTop(expected) => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value to assert");
                    }
                    let actual = self.stack.last().unwrap();
                    if (actual - expected).abs() > f64::EPSILON {
                        return Err("Assertion failed: value mismatch");
                    }
                }
                Op::DumpStack => {
                    println!("Stack contents (bottom to top):");
                    for (i, &value) in self.stack.iter().enumerate() {
                        println!("  [{}] {}", i, value);
                    }
                }
                Op::DumpMemory => {
                    println!("Memory contents:");
                    for (key, &value) in self.memory.iter() {
                        println!("  {} = {}", key, value);
                    }
                }
                Op::AssertMemory { key, expected } => {
                    if let Some(&value) = self.memory.get(key) {
                        if (value - expected).abs() > f64::EPSILON {
                            return Err("Assertion failed: memory value mismatch");
                        }
                    } else {
                        return Err("Key not found in memory");
                    }
                }
                Op::Pop => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value to pop");
                    }
                    self.stack.pop();
                }
                Op::Eq => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Eq");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(if a == b { 1.0 } else { 0.0 });
                }
                Op::Gt => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Gt");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(if a > b { 1.0 } else { 0.0 });
                }
                Op::Lt => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Lt");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(if a < b { 1.0 } else { 0.0 });
                }
                Op::Not => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value for Not");
                    }
                    let value = self.stack.pop().unwrap();
                    self.stack.push(if value == 0.0 { 1.0 } else { 0.0 });
                }
                Op::And => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for And");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 });
                }
                Op::Or => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Or");
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 });
                }
                Op::While { condition, body } => {
                    loop {
                        // Execute condition
                        self.execute(condition)?;
                        
                        // Check result
                        if self.stack.is_empty() {
                            return Err("Stack underflow: condition block must leave a value");
                        }
                        let result = self.stack.pop().unwrap();
                        
                        // Break if condition is false
                        if result == 0.0 {
                            break;
                        }
                        
                        // Execute body
                        self.execute(body)?;
                    }
                }
                Op::Dup => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value to duplicate");
                    }
                    let value = self.stack.last().unwrap();
                    self.stack.push(*value);
                }
                Op::Swap => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values to swap");
                    }
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    self.stack.push(a);
                    self.stack.push(b);
                }
                Op::Over => {
                    if self.stack.len() < 2 {
                        return Err("Stack underflow: need at least 2 values for Over");
                    }
                    let value = self.stack[self.stack.len() - 2];
                    self.stack.push(value);
                }
            }
        }
        Ok(())
    }

    pub fn top(&self) -> Option<f64> {
        self.stack.last().copied()
    }

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

        assert_eq!(vm.execute(&ops), Err("Division by zero"));
    }

    #[test]
    fn test_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![Op::Push(5.0), Op::Add];

        assert_eq!(
            vm.execute(&ops),
            Err("Stack underflow: need at least 2 values for Add")
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

        assert_eq!(vm.execute(&ops), Err("Key not found in memory"));
    }

    #[test]
    fn test_store_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::Store("x".to_string())];

        assert_eq!(
            vm.execute(&ops),
            Err("Stack underflow: need a value to store")
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
            Op::Push(0.0),
            Op::If {
                condition: vec![Op::Push(0.0)],
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
            Op::Push(1.0),
            Op::If {
                condition: vec![Op::Push(0.0)],
                then: vec![Op::Push(42.0)],
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(42.0));
    }

    #[test]
    fn test_nested_if_zero() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::If {
                condition: vec![
                    Op::Push(1.0),
                    Op::If {
                        condition: vec![Op::Push(0.0)],
                        then: vec![Op::Push(42.0)],
                        else_: None,
                    },
                ],
                then: vec![Op::Push(24.0)],
                else_: None,
            },
        ];

        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(24.0));
    }

    #[test]
    fn test_if_zero_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::If {
            condition: vec![Op::Push(0.0)],
            then: vec![Op::Push(42.0)],
            else_: None,
        }];

        assert_eq!(
            vm.execute(&ops),
            Err("Stack underflow: need a value for If")
        );
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value to negate"));
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
        
        assert_eq!(vm.execute(&ops), Err("Assertion failed: value mismatch"));
    }

    #[test]
    fn test_assert_top_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![Op::AssertTop(42.0)];
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value to assert"));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value for Not"));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values for And"));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values for Or"));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: condition block must leave a value"));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value to duplicate"));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values to swap"));
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
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values for Over"));
    }

    #[test]
    fn test_stack_manipulation_chain() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(1.0),
            Op::Push(2.0),
            Op::Push(3.0),
            Op::Dup,
            Op::Swap,
            Op::Over,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.stack, vec![1.0, 2.0, 3.0, 3.0, 2.0, 1.0]);
    }
}
