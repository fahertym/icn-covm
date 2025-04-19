//! Arithmetic Operations Implementation
//!
//! This module implements the ArithmeticOpHandler and ComparisonOpHandler traits
//! for the VM execution environment. It provides implementations for:
//! - Basic arithmetic operations (add, subtract, multiply, divide, modulo)
//! - Comparison operations (equals, greater than, less than)
//! - Logical operations (not, and, or)

use crate::typed::{TypedValue, TypedValueError};
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
    fn execute_arithmetic(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError> {
        match op {
            "add" => a.add(b).map_err(|e| VMError::TypedValueError(e)),
            "sub" => a.sub(b).map_err(|e| VMError::TypedValueError(e)),
            "mul" => a.mul(b).map_err(|e| VMError::TypedValueError(e)),
            "div" => a.div(b).map_err(|e| VMError::TypedValueError(e)),
            "mod" => a.modulo(b).map_err(|e| VMError::TypedValueError(e)),
            _ => Err(VMError::InvalidOperation {
                operation: op.to_string(),
            }),
        }
    }
}

impl ComparisonOpHandler for ArithmeticOpImpl {
    fn execute_comparison(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError> {
        match op {
            "eq" => a.equals(b).map_err(|e| VMError::TypedValueError(e)),
            "gt" => a.greater_than(b).map_err(|e| VMError::TypedValueError(e)),
            "lt" => a.less_than(b).map_err(|e| VMError::TypedValueError(e)),
            "gte" => {
                // A >= B is equivalent to !(A < B)
                let lt_result = a.less_than(b).map_err(|e| VMError::TypedValueError(e))?;
                lt_result.logical_not().map_err(|e| VMError::TypedValueError(e))
            },
            "lte" => {
                // A <= B is equivalent to !(A > B)
                let gt_result = a.greater_than(b).map_err(|e| VMError::TypedValueError(e))?;
                gt_result.logical_not().map_err(|e| VMError::TypedValueError(e))
            },
            "neq" => {
                // A != B is equivalent to !(A == B)
                let eq_result = a.equals(b).map_err(|e| VMError::TypedValueError(e))?;
                eq_result.logical_not().map_err(|e| VMError::TypedValueError(e))
            },
            _ => Err(VMError::InvalidOperation {
                operation: op.to_string(),
            }),
        }
    }

    fn execute_logical(&self, a: &TypedValue, op: &str) -> Result<TypedValue, VMError> {
        match op {
            "not" => a.logical_not().map_err(|e| VMError::TypedValueError(e)),
            _ => Err(VMError::InvalidOperation {
                operation: op.to_string(),
            }),
        }
    }

    fn execute_binary_logical(&self, a: &TypedValue, b: &TypedValue, op: &str) -> Result<TypedValue, VMError> {
        match op {
            "and" => a.logical_and(b).map_err(|e| VMError::TypedValueError(e)),
            "or" => a.logical_or(b).map_err(|e| VMError::TypedValueError(e)),
            "xor" => {
                // A XOR B = (A OR B) AND NOT (A AND B)
                let and_result = a.logical_and(b).map_err(|e| VMError::TypedValueError(e))?;
                let not_and = and_result.logical_not().map_err(|e| VMError::TypedValueError(e))?;
                let or_result = a.logical_or(b).map_err(|e| VMError::TypedValueError(e))?;
                or_result.logical_and(&not_and).map_err(|e| VMError::TypedValueError(e))
            },
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
        
        assert_eq!(
            arith.execute_arithmetic(&TypedValue::Number(3.0), &TypedValue::Number(2.0), "add").unwrap(),
            TypedValue::Number(5.0)
        );
        assert_eq!(
            arith.execute_arithmetic(&TypedValue::Number(3.0), &TypedValue::Number(2.0), "sub").unwrap(),
            TypedValue::Number(1.0)
        );
        assert_eq!(
            arith.execute_arithmetic(&TypedValue::Number(3.0), &TypedValue::Number(2.0), "mul").unwrap(),
            TypedValue::Number(6.0)
        );
        assert_eq!(
            arith.execute_arithmetic(&TypedValue::Number(6.0), &TypedValue::Number(2.0), "div").unwrap(),
            TypedValue::Number(3.0)
        );
        assert_eq!(
            arith.execute_arithmetic(&TypedValue::Number(7.0), &TypedValue::Number(2.0), "mod").unwrap(),
            TypedValue::Number(1.0)
        );
        
        // Test division by zero
        assert!(matches!(
            arith.execute_arithmetic(&TypedValue::Number(5.0), &TypedValue::Number(0.0), "div"),
            Err(VMError::TypedValueError(_))
        ));
        
        // Test string concatenation
        assert_eq!(
            arith.execute_arithmetic(&TypedValue::String("Hello, ".to_string()), &TypedValue::String("World!".to_string()), "add").unwrap(),
            TypedValue::String("Hello, World!".to_string())
        );
        
        // Test string and number
        assert_eq!(
            arith.execute_arithmetic(&TypedValue::String("Count: ".to_string()), &TypedValue::Number(42.0), "add").unwrap(),
            TypedValue::String("Count: 42".to_string())
        );
        
        // Test invalid operation
        assert!(matches!(
            arith.execute_arithmetic(&TypedValue::Number(1.0), &TypedValue::Number(2.0), "invalid"),
            Err(VMError::InvalidOperation { .. })
        ));
    }
    
    #[test]
    fn test_comparison_operations() {
        let arith = ArithmeticOpImpl::new();
        
        assert_eq!(
            arith.execute_comparison(&TypedValue::Number(2.0), &TypedValue::Number(2.0), "eq").unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            arith.execute_comparison(&TypedValue::Number(3.0), &TypedValue::Number(2.0), "eq").unwrap(),
            TypedValue::Boolean(false)
        );
        
        assert_eq!(
            arith.execute_comparison(&TypedValue::Number(3.0), &TypedValue::Number(2.0), "gt").unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            arith.execute_comparison(&TypedValue::Number(2.0), &TypedValue::Number(3.0), "gt").unwrap(),
            TypedValue::Boolean(false)
        );
        
        assert_eq!(
            arith.execute_comparison(&TypedValue::Number(2.0), &TypedValue::Number(3.0), "lt").unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            arith.execute_comparison(&TypedValue::Number(3.0), &TypedValue::Number(2.0), "lt").unwrap(),
            TypedValue::Boolean(false)
        );
        
        // Test string comparisons
        assert_eq!(
            arith.execute_comparison(&TypedValue::String("abc".to_string()), &TypedValue::String("def".to_string()), "lt").unwrap(),
            TypedValue::Boolean(true)
        );
        
        // Test invalid operation
        assert!(matches!(
            arith.execute_comparison(&TypedValue::Number(1.0), &TypedValue::Number(2.0), "invalid"),
            Err(VMError::InvalidOperation { .. })
        ));
    }
    
    #[test]
    fn test_logical_operations() {
        let arith = ArithmeticOpImpl::new();
        
        assert_eq!(
            arith.execute_logical(&TypedValue::Boolean(false), "not").unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            arith.execute_logical(&TypedValue::Boolean(true), "not").unwrap(),
            TypedValue::Boolean(false)
        );
        assert_eq!(
            arith.execute_logical(&TypedValue::Number(0.0), "not").unwrap(),
            TypedValue::Boolean(true)
        );
        
        // Test invalid operation
        assert!(matches!(
            arith.execute_logical(&TypedValue::Number(1.0), "invalid"),
            Err(VMError::InvalidOperation { .. })
        ));
    }
    
    #[test]
    fn test_binary_logical_operations() {
        let arith = ArithmeticOpImpl::new();
        
        // AND
        assert_eq!(
            arith.execute_binary_logical(&TypedValue::Boolean(true), &TypedValue::Boolean(true), "and").unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            arith.execute_binary_logical(&TypedValue::Boolean(true), &TypedValue::Boolean(false), "and").unwrap(),
            TypedValue::Boolean(false)
        );
        
        // OR
        assert_eq!(
            arith.execute_binary_logical(&TypedValue::Boolean(true), &TypedValue::Boolean(false), "or").unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            arith.execute_binary_logical(&TypedValue::Boolean(false), &TypedValue::Boolean(false), "or").unwrap(),
            TypedValue::Boolean(false)
        );
        
        // Test with Numbers (should coerce to boolean)
        assert_eq!(
            arith.execute_binary_logical(&TypedValue::Number(1.0), &TypedValue::Number(0.0), "and").unwrap(),
            TypedValue::Boolean(false)
        );
        
        // Test invalid operation
        assert!(matches!(
            arith.execute_binary_logical(&TypedValue::Number(1.0), &TypedValue::Number(2.0), "invalid"),
            Err(VMError::InvalidOperation { .. })
        ));
    }
} 