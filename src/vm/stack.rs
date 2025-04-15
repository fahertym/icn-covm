//! VM Stack operations
//! 
//! This module provides stack manipulation operations for the VM.

use crate::vm::errors::VMError;

/// Provides stack operations for the virtual machine
#[derive(Debug, Clone)]
pub struct VMStack {
    /// The values on the stack
    stack: Vec<f64>,
}

impl VMStack {
    /// Create a new empty stack
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push a value onto the stack
    pub fn push(&mut self, value: f64) {
        self.stack.push(value);
    }

    /// Pop a value from the stack
    pub fn pop(&mut self, op_name: &str) -> Result<f64, VMError> {
        self.stack.pop().ok_or_else(|| VMError::StackUnderflow {
            op_name: op_name.to_string(),
        })
    }

    /// Pop two values from the stack
    pub fn pop_two(&mut self, op_name: &str) -> Result<(f64, f64), VMError> {
        let b = self.pop(op_name)?;
        let a = self.pop(op_name)?;
        Ok((a, b))
    }

    /// Return the top value from the stack without popping it
    pub fn top(&self) -> Option<f64> {
        self.stack.last().copied()
    }

    /// Get the current stack values
    pub fn get_stack(&self) -> Vec<f64> {
        self.stack.clone()
    }

    /// Duplicate the top value on the stack
    pub fn dup(&mut self, op_name: &str) -> Result<(), VMError> {
        let value = self.top().ok_or_else(|| VMError::StackUnderflow {
            op_name: op_name.to_string(),
        })?;
        self.push(value);
        Ok(())
    }

    /// Swap the top two values on the stack
    pub fn swap(&mut self, op_name: &str) -> Result<(), VMError> {
        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow {
                op_name: op_name.to_string(),
            });
        }
        
        let len = self.stack.len();
        self.stack.swap(len - 1, len - 2);
        Ok(())
    }

    /// Copy the second value to the top of the stack
    pub fn over(&mut self, op_name: &str) -> Result<(), VMError> {
        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow {
                op_name: op_name.to_string(),
            });
        }
        
        let len = self.stack.len();
        let value = self.stack[len - 2];
        self.push(value);
        Ok(())
    }

    /// Check if all values in the specified depth are equal
    pub fn assert_equal_stack(&self, depth: usize, op_name: &str) -> Result<bool, VMError> {
        if self.stack.len() < depth {
            return Err(VMError::StackUnderflow {
                op_name: op_name.to_string(),
            });
        }

        let start_idx = self.stack.len() - depth;
        let first_val = self.stack[start_idx];
        
        for i in (start_idx + 1)..self.stack.len() {
            if (self.stack[i] - first_val).abs() > f64::EPSILON {
                return Ok(false);
            }
        }
        
        Ok(true)
    }

    /// Format the stack as a string for display
    pub fn format_stack(&self) -> String {
        if self.stack.is_empty() {
            return "Stack: []".to_string();
        }
        
        let items: Vec<String> = self.stack.iter().map(|v| v.to_string()).collect();
        format!("Stack: [{}]", items.join(", "))
    }

    /// Clear the stack
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Get the stack length
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let mut stack = VMStack::new();
        stack.push(42.0);
        assert_eq!(stack.pop("test").unwrap(), 42.0);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_pop_empty() {
        let mut stack = VMStack::new();
        let result = stack.pop("test");
        assert!(matches!(result, Err(VMError::StackUnderflow { .. })));
    }

    #[test]
    fn test_stack_operations() {
        let mut stack = VMStack::new();
        stack.push(1.0);
        stack.push(2.0);
        
        // Test dup
        stack.dup("dup").unwrap();
        assert_eq!(stack.get_stack(), vec![1.0, 2.0, 2.0]);
        
        // Test swap
        stack.swap("swap").unwrap();
        assert_eq!(stack.get_stack(), vec![1.0, 2.0, 2.0]);
        
        // Test over
        stack.over("over").unwrap();
        assert_eq!(stack.get_stack(), vec![1.0, 2.0, 2.0, 2.0]);
        
        // Test assert_equal_stack
        assert!(stack.assert_equal_stack(3, "assert").unwrap());
        stack.push(3.0);
        assert!(!stack.assert_equal_stack(2, "assert").unwrap());
    }
} 