//! VM Stack operations
//!
//! This module provides stack manipulation operations for the VM.
//!
//! The stack is a fundamental component of the VM, responsible for:
//! - Storing intermediate computation results
//! - Providing operands for operations
//! - Managing function call parameters and return values
//!
//! This module is separated from other VM components to:
//! - Enable clear focus on stack operations without other concerns
//! - Allow for independent testing of stack functionality
//! - Provide a clean interface for the main VM implementation
//! - Support potential future optimizations specific to stack operations
//!
//! The module defines a `StackOps` trait that encapsulates the operations
//! that can be performed on a stack, enabling alternative stack implementations
//! if needed in the future.

use crate::typed::{TypedValue, TypedValueError};
use crate::vm::errors::VMError;

/// Defines operations that can be performed on a stack
pub trait StackOps {
    /// Push a value onto the stack
    fn push(&mut self, value: TypedValue);

    /// Pop a value from the stack
    fn pop(&mut self, op_name: &str) -> Result<TypedValue, VMError>;

    /// Pop two values from the stack
    fn pop_two(&mut self, op_name: &str) -> Result<(TypedValue, TypedValue), VMError>;

    /// Return the top value from the stack without popping it
    fn top(&self) -> Option<&TypedValue>;

    /// Get the current stack values
    fn get_stack(&self) -> Vec<TypedValue>;

    /// Duplicate the top value on the stack
    fn dup(&mut self, op_name: &str) -> Result<(), VMError>;

    /// Swap the top two values on the stack
    fn swap(&mut self, op_name: &str) -> Result<(), VMError>;

    /// Copy the second value to the top of the stack
    fn over(&mut self, op_name: &str) -> Result<(), VMError>;

    /// Check if all values in the specified depth are equal
    fn assert_equal_stack(&self, depth: usize, op_name: &str) -> Result<bool, VMError>;

    /// Format the stack as a string for display
    fn format_stack(&self) -> String;

    /// Clear the stack
    fn clear(&mut self);

    /// Get the stack length
    fn len(&self) -> usize;

    /// Check if the stack is empty
    fn is_empty(&self) -> bool;

    /// Pop a number from the stack, with type checking
    fn pop_number(&mut self, op_name: &str) -> Result<f64, VMError>;

    /// Pop a boolean from the stack, with type checking
    fn pop_bool(&mut self, op_name: &str) -> Result<bool, VMError>;

    /// Pop a string from the stack, with type checking
    fn pop_string(&mut self, op_name: &str) -> Result<String, VMError>;

    /// Peek the type of the top value on the stack
    fn peek_type(&self) -> Option<&TypedValue>;
}

/// Provides stack operations for the virtual machine
#[derive(Debug, Clone)]
pub struct VMStack {
    /// The values on the stack
    stack: Vec<TypedValue>,
}

impl VMStack {
    /// Create a new empty stack
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }
}

impl StackOps for VMStack {
    /// Push a value onto the stack
    fn push(&mut self, value: TypedValue) {
        self.stack.push(value);
    }

    /// Pop a value from the stack
    fn pop(&mut self, _op_name: &str) -> Result<TypedValue, VMError> {
        self.stack.pop().ok_or(VMError::StackUnderflow)
    }

    /// Pop two values from the stack
    fn pop_two(&mut self, op_name: &str) -> Result<(TypedValue, TypedValue), VMError> {
        let b = self.pop(op_name)?;
        let a = self.pop(op_name)?;
        Ok((a, b))
    }

    /// Return the top value from the stack without popping it
    fn top(&self) -> Option<&TypedValue> {
        self.stack.last()
    }

    /// Get the current stack values
    fn get_stack(&self) -> Vec<TypedValue> {
        self.stack.clone()
    }

    /// Duplicate the top value on the stack
    fn dup(&mut self, _op_name: &str) -> Result<(), VMError> {
        let value = self.top().ok_or(VMError::StackUnderflow)?.clone();
        self.push(value);
        Ok(())
    }

    /// Swap the top two values on the stack
    fn swap(&mut self, _op_name: &str) -> Result<(), VMError> {
        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow);
        }

        let len = self.stack.len();
        self.stack.swap(len - 1, len - 2);
        Ok(())
    }

    /// Copy the second value to the top of the stack
    fn over(&mut self, _op_name: &str) -> Result<(), VMError> {
        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow);
        }

        let len = self.stack.len();
        let value = self.stack[len - 2].clone();
        self.push(value);
        Ok(())
    }

    /// Check if all values in the specified depth are equal
    fn assert_equal_stack(&self, depth: usize, op_name: &str) -> Result<bool, VMError> {
        if self.stack.len() < depth {
            return Err(VMError::StackUnderflow);
        }

        let top_value = self.stack.last().unwrap();
        let mut all_equal = true;

        for i in 1..depth {
            let index = self.stack.len() - 1 - i;
            if self.stack[index] != *top_value {
                all_equal = false;
                break;
            }
        }

        Ok(all_equal)
    }

    /// Format the stack as a string for display
    fn format_stack(&self) -> String {
        let mut result = String::new();
        for (i, value) in self.stack.iter().enumerate() {
            result.push_str(&format!("{}: {}\n", i, value));
        }
        result
    }

    /// Clear the stack
    fn clear(&mut self) {
        self.stack.clear();
    }

    /// Get the stack length
    fn len(&self) -> usize {
        self.stack.len()
    }

    /// Check if the stack is empty
    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Pop a number from the stack, with type checking
    fn pop_number(&mut self, op_name: &str) -> Result<f64, VMError> {
        let value = self.pop(op_name)?;
        value.as_number().map_err(|_| VMError::TypeMismatch {
            expected: "Number".to_string(),
            found: value.type_name().to_string(),
            operation: op_name.to_string(),
        })
    }

    /// Pop a boolean from the stack, with type checking
    fn pop_bool(&mut self, op_name: &str) -> Result<bool, VMError> {
        let value = self.pop(op_name)?;
        value.as_boolean().map_err(|_| VMError::TypeMismatch {
            expected: "Boolean".to_string(),
            found: value.type_name().to_string(),
            operation: op_name.to_string(),
        })
    }

    /// Pop a string from the stack, with type checking
    fn pop_string(&mut self, op_name: &str) -> Result<String, VMError> {
        let value = self.pop(op_name)?;
        value.as_string().map_err(|_| VMError::TypeMismatch {
            expected: "String".to_string(),
            found: value.type_name().to_string(),
            operation: op_name.to_string(),
        })
    }

    /// Peek the type of the top value on the stack
    fn peek_type(&self) -> Option<&TypedValue> {
        self.stack.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_pop() {
        let mut stack = VMStack::new();
        stack.push(TypedValue::Number(10.0));
        stack.push(TypedValue::Number(20.0));

        assert_eq!(stack.len(), 2);
        assert_eq!(stack.pop("test").unwrap(), TypedValue::Number(20.0));
        assert_eq!(stack.pop("test").unwrap(), TypedValue::Number(10.0));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_pop_two() {
        let mut stack = VMStack::new();
        stack.push(TypedValue::Number(10.0));
        stack.push(TypedValue::Number(20.0));

        let (a, b) = stack.pop_two("test").unwrap();
        assert_eq!(a, TypedValue::Number(10.0));
        assert_eq!(b, TypedValue::Number(20.0));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_typed_stack_operations() {
        let mut stack = VMStack::new();

        // Push different types
        stack.push(TypedValue::Number(10.0));
        stack.push(TypedValue::Boolean(true));
        stack.push(TypedValue::String("hello".to_string()));

        // Check type-specific pops
        assert_eq!(stack.pop_string("test").unwrap(), "hello");
        assert_eq!(stack.pop_bool("test").unwrap(), true);
        assert_eq!(stack.pop_number("test").unwrap(), 10.0);

        // Test type mismatch error
        stack.push(TypedValue::String("not a number".to_string()));
        let err = stack.pop_number("test").unwrap_err();
        match err {
            VMError::TypeMismatch {
                expected, found, ..
            } => {
                assert_eq!(expected, "Number");
                assert_eq!(found, "String");
            }
            _ => panic!("Expected TypeMismatch error"),
        }
    }

    #[test]
    fn test_dup_and_swap() {
        let mut stack = VMStack::new();
        stack.push(TypedValue::Number(10.0));

        stack.dup("test").unwrap();
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.top().unwrap(), &TypedValue::Number(10.0));

        stack.push(TypedValue::String("hello".to_string()));
        stack.swap("test").unwrap();

        assert_eq!(stack.pop("test").unwrap(), TypedValue::Number(10.0));
        assert_eq!(
            stack.pop("test").unwrap(),
            TypedValue::String("hello".to_string())
        );
    }
}
