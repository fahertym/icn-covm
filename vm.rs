#[derive(Debug)]
pub enum Op {
    Push(f64),
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug)]
pub struct VM {
    stack: Vec<f64>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
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
            }
        }
        Ok(())
    }

    pub fn top(&self) -> Option<f64> {
        self.stack.last().copied()
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
} 