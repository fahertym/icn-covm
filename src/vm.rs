use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Op {
    Push(f64),
    Add,
    Sub,
    Mul,
    Div,
    Store(String),
    Load(String),
    IfZero {
        then: Vec<Op>,
        else_: Vec<Op>,
    },
    Loop {
        count: usize,
        body: Vec<Op>,
    },
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
                Op::IfZero { then, else_ } => {
                    if self.stack.is_empty() {
                        return Err("Stack underflow: need a value for IfZero");
                    }
                    let condition = self.stack.pop().unwrap();
                    if condition == 0.0 {
                        self.execute(then)?;
                    } else {
                        self.execute(else_)?;
                    }
                }
                Op::Loop { count, body } => {
                    for _ in 0..*count {
                        self.execute(body)?;
                    }
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
        let ops = vec![
            Op::Push(10.0),
            Op::Push(2.0),
            Op::Div,
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(5.0));
    }

    #[test]
    fn test_division_by_zero() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(10.0),
            Op::Push(0.0),
            Op::Div,
        ];
        
        assert_eq!(vm.execute(&ops), Err("Division by zero"));
    }

    #[test]
    fn test_stack_underflow() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(5.0),
            Op::Add,
        ];
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need at least 2 values for Add"));
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
        let ops = vec![
            Op::Load("nonexistent".to_string()),
        ];
        
        assert_eq!(vm.execute(&ops), Err("Key not found in memory"));
    }

    #[test]
    fn test_store_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Store("x".to_string()),
        ];
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value to store"));
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
            Op::IfZero {
                then: vec![Op::Push(42.0)],
                else_: vec![Op::Push(24.0)],
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
            Op::IfZero {
                then: vec![Op::Push(42.0)],
                else_: vec![Op::Push(24.0)],
            },
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(24.0));
    }

    #[test]
    fn test_nested_if_zero() {
        let mut vm = VM::new();
        let ops = vec![
            Op::Push(0.0),
            Op::IfZero {
                then: vec![
                    Op::Push(1.0),
                    Op::IfZero {
                        then: vec![Op::Push(42.0)],
                        else_: vec![Op::Push(24.0)],
                    },
                ],
                else_: vec![Op::Push(100.0)],
            },
        ];
        
        assert!(vm.execute(&ops).is_ok());
        assert_eq!(vm.top(), Some(24.0));
    }

    #[test]
    fn test_if_zero_empty_stack() {
        let mut vm = VM::new();
        let ops = vec![
            Op::IfZero {
                then: vec![Op::Push(42.0)],
                else_: vec![Op::Push(24.0)],
            },
        ];
        
        assert_eq!(vm.execute(&ops), Err("Stack underflow: need a value for IfZero"));
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
                body: vec![
                    Op::Push(100.0),
                    Op::Store("value".to_string()),
                ],
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
} 