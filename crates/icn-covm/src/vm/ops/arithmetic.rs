//! Arithmetic Operations Implementation
//!
//! This module implements the ArithmeticOpHandler and ComparisonOpHandler traits
//! for the VM execution environment. It provides implementations for:
//! - Basic arithmetic operations (add, subtract, multiply, divide, modulo)
//! - Comparison operations (equals, greater than, less than)
//! - Logical operations (not, and, or)

use crate::vm::errors::VMError;
use crate::vm::ops::{ArithmeticOpHandler, ComparisonOpHandler};

/// Implementation of arithmetic and comparison operations for the VM
#[derive(Debug, Clone, Default)]
pub struct ArithmeticOpImpl;

impl ArithmeticOpImpl {
    /// Create a new arithmetic operations handler
    pub fn new() -> Self {
        Self {}
    }
}

impl ArithmeticOpHandler for ArithmeticOpImpl {
    fn execute_arithmetic(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError> {
        match op {
            "add" => Ok(a + b),
            "sub" => Ok(a - b),
            "mul" => Ok(a * b),
            "div" => {
                if b == 0.0 {
                    Err(VMError::DivisionByZero)
                } else {
                    Ok(a / b)
                }
            }
            "mod" => {
                if b == 0.0 {
                    Err(VMError::DivisionByZero)
                } else {
                    Ok(a % b)
                }
            }
            _ => Err(VMError::InvalidOperation {
                operation: op.to_string(),
            }),
        }
    }
}

impl ComparisonOpHandler for ArithmeticOpImpl {
    fn execute_comparison(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError> {
        match op {
            "eq" => Ok(if (a - b).abs() < f64::EPSILON { 1.0 } else { 0.0 }),
            "gt" => Ok(if a > b { 1.0 } else { 0.0 }),
            "lt" => Ok(if a < b { 1.0 } else { 0.0 }),
            "gte" => Ok(if a >= b { 1.0 } else { 0.0 }),
            "lte" => Ok(if a <= b { 1.0 } else { 0.0 }),
            "neq" => Ok(if (a - b).abs() >= f64::EPSILON { 1.0 } else { 0.0 }),
            _ => Err(VMError::InvalidOperation {
                operation: op.to_string(),
            }),
        }
    }

    fn execute_logical(&self, a: f64, op: &str) -> Result<f64, VMError> {
        match op {
            "not" => Ok(if a == 0.0 { 1.0 } else { 0.0 }),
            _ => Err(VMError::InvalidOperation {
                operation: op.to_string(),
            }),
        }
    }

    fn execute_binary_logical(&self, a: f64, b: f64, op: &str) -> Result<f64, VMError> {
        match op {
            "and" => Ok(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 }),
            "or" => Ok(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 }),
            "xor" => Ok(if (a != 0.0) != (b != 0.0) { 1.0 } else { 0.0 }),
            _ => Err(VMError::InvalidOperation {
                operation: op.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic_operations() {
        let arith = ArithmeticOpImpl::new();
        
        assert_eq!(arith.execute_arithmetic(3.0, 2.0, "add").unwrap(), 5.0);
        assert_eq!(arith.execute_arithmetic(3.0, 2.0, "sub").unwrap(), 1.0);
        assert_eq!(arith.execute_arithmetic(3.0, 2.0, "mul").unwrap(), 6.0);
        assert_eq!(arith.execute_arithmetic(6.0, 2.0, "div").unwrap(), 3.0);
        assert_eq!(arith.execute_arithmetic(7.0, 2.0, "mod").unwrap(), 1.0);
        
        // Test division by zero
        assert!(matches!(
            arith.execute_arithmetic(5.0, 0.0, "div"),
            Err(VMError::DivisionByZero)
        ));
        
        // Test modulo by zero
        assert!(matches!(
            arith.execute_arithmetic(5.0, 0.0, "mod"),
            Err(VMError::DivisionByZero)
        ));
        
        // Test invalid operation
        assert!(matches!(
            arith.execute_arithmetic(1.0, 2.0, "invalid"),
            Err(VMError::InvalidOperation { .. })
        ));
    }
    
    #[test]
    fn test_comparison_operations() {
        let arith = ArithmeticOpImpl::new();
        
        assert_eq!(arith.execute_comparison(2.0, 2.0, "eq").unwrap(), 1.0);
        assert_eq!(arith.execute_comparison(3.0, 2.0, "eq").unwrap(), 0.0);
        
        assert_eq!(arith.execute_comparison(3.0, 2.0, "gt").unwrap(), 1.0);
        assert_eq!(arith.execute_comparison(2.0, 3.0, "gt").unwrap(), 0.0);
        
        assert_eq!(arith.execute_comparison(2.0, 3.0, "lt").unwrap(), 1.0);
        assert_eq!(arith.execute_comparison(3.0, 2.0, "lt").unwrap(), 0.0);
        
        assert_eq!(arith.execute_comparison(3.0, 3.0, "gte").unwrap(), 1.0);
        assert_eq!(arith.execute_comparison(4.0, 3.0, "gte").unwrap(), 1.0);
        assert_eq!(arith.execute_comparison(2.0, 3.0, "gte").unwrap(), 0.0);
        
        assert_eq!(arith.execute_comparison(3.0, 3.0, "lte").unwrap(), 1.0);
        assert_eq!(arith.execute_comparison(2.0, 3.0, "lte").unwrap(), 1.0);
        assert_eq!(arith.execute_comparison(4.0, 3.0, "lte").unwrap(), 0.0);
        
        assert_eq!(arith.execute_comparison(2.0, 3.0, "neq").unwrap(), 1.0);
        assert_eq!(arith.execute_comparison(3.0, 3.0, "neq").unwrap(), 0.0);
        
        // Test invalid operation
        assert!(matches!(
            arith.execute_comparison(1.0, 2.0, "invalid"),
            Err(VMError::InvalidOperation { .. })
        ));
    }
    
    #[test]
    fn test_logical_operations() {
        let arith = ArithmeticOpImpl::new();
        
        assert_eq!(arith.execute_logical(0.0, "not").unwrap(), 1.0);
        assert_eq!(arith.execute_logical(1.0, "not").unwrap(), 0.0);
        assert_eq!(arith.execute_logical(42.0, "not").unwrap(), 0.0);
        
        // Test invalid operation
        assert!(matches!(
            arith.execute_logical(1.0, "invalid"),
            Err(VMError::InvalidOperation { .. })
        ));
    }
    
    #[test]
    fn test_binary_logical_operations() {
        let arith = ArithmeticOpImpl::new();
        
        // AND
        assert_eq!(arith.execute_binary_logical(1.0, 1.0, "and").unwrap(), 1.0);
        assert_eq!(arith.execute_binary_logical(1.0, 0.0, "and").unwrap(), 0.0);
        assert_eq!(arith.execute_binary_logical(0.0, 1.0, "and").unwrap(), 0.0);
        assert_eq!(arith.execute_binary_logical(0.0, 0.0, "and").unwrap(), 0.0);
        
        // OR
        assert_eq!(arith.execute_binary_logical(1.0, 1.0, "or").unwrap(), 1.0);
        assert_eq!(arith.execute_binary_logical(1.0, 0.0, "or").unwrap(), 1.0);
        assert_eq!(arith.execute_binary_logical(0.0, 1.0, "or").unwrap(), 1.0);
        assert_eq!(arith.execute_binary_logical(0.0, 0.0, "or").unwrap(), 0.0);
        
        // XOR
        assert_eq!(arith.execute_binary_logical(1.0, 1.0, "xor").unwrap(), 0.0);
        assert_eq!(arith.execute_binary_logical(1.0, 0.0, "xor").unwrap(), 1.0);
        assert_eq!(arith.execute_binary_logical(0.0, 1.0, "xor").unwrap(), 1.0);
        assert_eq!(arith.execute_binary_logical(0.0, 0.0, "xor").unwrap(), 0.0);
        
        // Test invalid operation
        assert!(matches!(
            arith.execute_binary_logical(1.0, 2.0, "invalid"),
            Err(VMError::InvalidOperation { .. })
        ));
    }
} 