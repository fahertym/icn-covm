use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// Errors specific to typed value operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TypedValueError {
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("Invalid operation {op} for types {types}")]
    InvalidOperationForType { op: String, types: String },

    #[error("Cannot coerce from {from} to {to}")]
    CoercionError { from: String, to: String },

    #[error("Value out of bounds")]
    ValueOutOfBounds,
}

/// A typed value that can be stored on the VM stack
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypedValue {
    Number(f64),
    Boolean(bool),
    String(String),
    Null,
}

impl TypedValue {
    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self {
            TypedValue::Number(_) => "Number",
            TypedValue::Boolean(_) => "Boolean",
            TypedValue::String(_) => "String",
            TypedValue::Null => "Null",
        }
    }

    /// Check if a value is considered falsey in boolean context
    /// - Numbers: 0.0 is falsey, any other number is truthy
    /// - Booleans: false is falsey, true is truthy
    /// - Strings: empty string is falsey, any other string is truthy
    /// - Null: always falsey
    pub fn is_falsey(&self) -> bool {
        match self {
            TypedValue::Number(n) => *n == 0.0,
            TypedValue::Boolean(b) => !b,
            TypedValue::String(s) => s.is_empty(),
            TypedValue::Null => true,
        }
    }

    /// Try to convert the value to a number
    pub fn as_number(&self) -> Result<f64, TypedValueError> {
        match self {
            TypedValue::Number(n) => Ok(*n),
            TypedValue::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
            TypedValue::String(s) => s
                .parse::<f64>()
                .map_err(|_| TypedValueError::CoercionError {
                    from: "String".to_string(),
                    to: "Number".to_string(),
                }),
            TypedValue::Null => Ok(0.0),
        }
    }

    /// Try to convert the value to a boolean
    pub fn as_boolean(&self) -> Result<bool, TypedValueError> {
        match self {
            TypedValue::Number(n) => Ok(*n != 0.0),
            TypedValue::Boolean(b) => Ok(*b),
            TypedValue::String(s) => Ok(!s.is_empty()),
            TypedValue::Null => Ok(false),
        }
    }

    /// Try to convert the value to a string
    pub fn as_string(&self) -> Result<String, TypedValueError> {
        match self {
            TypedValue::Number(n) => Ok(n.to_string()),
            TypedValue::Boolean(b) => Ok(b.to_string()),
            TypedValue::String(s) => Ok(s.clone()),
            TypedValue::Null => Ok("null".to_string()),
        }
    }

    /// Add two values, with type coercion
    pub fn add(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        match (self, other) {
            (TypedValue::Number(a), TypedValue::Number(b)) => Ok(TypedValue::Number(a + b)),
            (TypedValue::String(a), TypedValue::String(b)) => {
                Ok(TypedValue::String(format!("{}{}", a, b)))
            }
            (TypedValue::String(a), b) => {
                let b_str = b.as_string()?;
                Ok(TypedValue::String(format!("{}{}", a, b_str)))
            }
            (a, TypedValue::String(b)) => {
                let a_str = a.as_string()?;
                Ok(TypedValue::String(format!("{}{}", a_str, b)))
            }
            _ => {
                // Try numeric coercion for other combinations
                let a_num = self.as_number()?;
                let b_num = other.as_number()?;
                Ok(TypedValue::Number(a_num + b_num))
            }
        }
    }

    /// Subtract two values, with type coercion
    pub fn sub(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        // Subtraction requires numeric coercion
        let a_num = self.as_number()?;
        let b_num = other.as_number()?;
        Ok(TypedValue::Number(a_num - b_num))
    }

    /// Multiply two values, with type coercion
    pub fn mul(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        match (self, other) {
            (TypedValue::Number(a), TypedValue::Number(b)) => Ok(TypedValue::Number(a * b)),
            (TypedValue::String(s), TypedValue::Number(n))
            | (TypedValue::Number(n), TypedValue::String(s)) => {
                // String repetition
                let repeat = *n as usize;
                if repeat > 1000 {
                    // Avoid excessive memory allocation
                    return Err(TypedValueError::ValueOutOfBounds);
                }
                Ok(TypedValue::String(s.repeat(repeat)))
            }
            _ => {
                // Try numeric coercion for other combinations
                let a_num = self.as_number()?;
                let b_num = other.as_number()?;
                Ok(TypedValue::Number(a_num * b_num))
            }
        }
    }

    /// Divide two values, with type coercion
    pub fn div(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        // Division requires numeric coercion
        let a_num = self.as_number()?;
        let b_num = other.as_number()?;

        if b_num == 0.0 {
            return Err(TypedValueError::InvalidOperationForType {
                op: "division".to_string(),
                types: "by zero".to_string(),
            });
        }

        Ok(TypedValue::Number(a_num / b_num))
    }

    /// Modulo operation, with type coercion
    pub fn modulo(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        // Modulo requires numeric coercion
        let a_num = self.as_number()?;
        let b_num = other.as_number()?;

        if b_num == 0.0 {
            return Err(TypedValueError::InvalidOperationForType {
                op: "modulo".to_string(),
                types: "by zero".to_string(),
            });
        }

        Ok(TypedValue::Number(a_num % b_num))
    }

    /// Compare two values for equality
    pub fn equals(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        match (self, other) {
            (TypedValue::Number(a), TypedValue::Number(b)) => {
                Ok(TypedValue::Boolean((a - b).abs() < f64::EPSILON))
            }
            (TypedValue::Boolean(a), TypedValue::Boolean(b)) => Ok(TypedValue::Boolean(a == b)),
            (TypedValue::String(a), TypedValue::String(b)) => Ok(TypedValue::Boolean(a == b)),
            (TypedValue::Null, TypedValue::Null) => Ok(TypedValue::Boolean(true)),
            (TypedValue::Null, _) | (_, TypedValue::Null) => Ok(TypedValue::Boolean(false)),
            _ => {
                // For mixed types, try string comparison as a last resort
                let a_str = self.as_string()?;
                let b_str = other.as_string()?;
                Ok(TypedValue::Boolean(a_str == b_str))
            }
        }
    }

    /// Greater than comparison
    pub fn greater_than(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        match (self, other) {
            (TypedValue::Number(a), TypedValue::Number(b)) => Ok(TypedValue::Boolean(a > b)),
            (TypedValue::String(a), TypedValue::String(b)) => Ok(TypedValue::Boolean(a > b)),
            _ => {
                // For mixed types, try numeric comparison
                let a_num = self.as_number()?;
                let b_num = other.as_number()?;
                Ok(TypedValue::Boolean(a_num > b_num))
            }
        }
    }

    /// Less than comparison
    pub fn less_than(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        match (self, other) {
            (TypedValue::Number(a), TypedValue::Number(b)) => Ok(TypedValue::Boolean(a < b)),
            (TypedValue::String(a), TypedValue::String(b)) => Ok(TypedValue::Boolean(a < b)),
            _ => {
                // For mixed types, try numeric comparison
                let a_num = self.as_number()?;
                let b_num = other.as_number()?;
                Ok(TypedValue::Boolean(a_num < b_num))
            }
        }
    }

    /// Logical NOT operation
    pub fn logical_not(&self) -> Result<TypedValue, TypedValueError> {
        let b = self.as_boolean()?;
        Ok(TypedValue::Boolean(!b))
    }

    /// Logical AND operation
    pub fn logical_and(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        let a = self.as_boolean()?;
        let b = other.as_boolean()?;
        Ok(TypedValue::Boolean(a && b))
    }

    /// Logical OR operation
    pub fn logical_or(&self, other: &TypedValue) -> Result<TypedValue, TypedValueError> {
        let a = self.as_boolean()?;
        let b = other.as_boolean()?;
        Ok(TypedValue::Boolean(a || b))
    }

    /// Returns a human-readable description of the TypedValue for debugging
    pub fn describe(&self) -> String {
        match self {
            TypedValue::Number(n) => format!("Number({})", n),
            TypedValue::Boolean(b) => format!("Boolean({})", b),
            TypedValue::String(s) => format!("String(\"{}\")", s),
            TypedValue::Null => "Null".into(),
        }
    }
}

impl fmt::Display for TypedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypedValue::Number(n) => write!(f, "{}", n),
            TypedValue::Boolean(b) => write!(f, "{}", b),
            TypedValue::String(s) => write!(f, "\"{}\"", s),
            TypedValue::Null => write!(f, "null"),
        }
    }
}

/// Extended VM with typed values
#[derive(Debug)]
pub struct TypedVM {
    pub stack: Vec<TypedValue>,
    memory: HashMap<String, TypedValue>,
    functions: HashMap<String, (Vec<String>, Vec<crate::vm::Op>)>,
    call_frames: Vec<TypedCallFrame>,
    recursion_depth: usize,
    loop_control: TypedLoopControl,
}

#[derive(Debug)]
struct TypedCallFrame {
    memory: HashMap<String, TypedValue>,
    return_value: Option<TypedValue>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TypedLoopControl {
    None,
    Break,
    Continue,
}

impl TypedVM {
    /// Create a new typed VM
    pub fn new() -> Self {
        TypedVM {
            stack: Vec::new(),
            memory: HashMap::new(),
            functions: HashMap::new(),
            call_frames: Vec::new(),
            recursion_depth: 0,
            loop_control: TypedLoopControl::None,
        }
    }

    /// Get a reference to the stack
    pub fn get_stack(&self) -> &[TypedValue] {
        &self.stack
    }

    /// Get a value from memory
    pub fn get_memory(&self, key: &str) -> Option<&TypedValue> {
        self.memory.get(key)
    }

    /// Get a reference to the memory map
    pub fn get_memory_map(&self) -> &HashMap<String, TypedValue> {
        &self.memory
    }

    /// Set parameters for the VM
    pub fn set_parameters(
        &mut self,
        params: HashMap<String, String>,
    ) -> Result<(), crate::vm::VMError> {
        for (key, value) in params {
            // Try to parse as f64 first
            if let Ok(num) = value.parse::<f64>() {
                self.memory.insert(key, TypedValue::Number(num));
            } else if value == "true" || value == "false" {
                let bool_val = value == "true";
                self.memory.insert(key, TypedValue::Boolean(bool_val));
            } else {
                // Store as string
                self.memory.insert(key, TypedValue::String(value));
            }
        }
        Ok(())
    }

    /// Get the top value from the stack without removing it
    pub fn top(&self) -> Option<&TypedValue> {
        self.stack.last()
    }

    /// Helper for stack operations that need to pop one value
    fn pop_one(&mut self, op_name: &str) -> Result<TypedValue, crate::vm::VMError> {
        self.stack
            .pop()
            .ok_or_else(|| crate::vm::VMError::StackUnderflow {
                op_name: op_name.to_string(),
            })
    }

    /// Helper for stack operations that need to pop two values
    fn pop_two(&mut self, op_name: &str) -> Result<(TypedValue, TypedValue), crate::vm::VMError> {
        if self.stack.len() < 2 {
            return Err(crate::vm::VMError::StackUnderflow {
                op_name: op_name.to_string(),
            });
        }

        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        Ok((a, b))
    }

    /// Execute a list of operations
    pub fn execute(&mut self, ops: &[crate::vm::Op]) -> Result<(), crate::vm::VMError> {
        if self.recursion_depth > 1000 {
            return Err(crate::vm::VMError::MaxRecursionDepth);
        }

        // Implementation will be similar to VM::execute_inner but with typed values
        // This would be a much longer implementation, but for brevity we'll just
        // show the high-level structure

        for op in ops {
            match op {
                crate::vm::Op::Push(val) => {
                    self.stack.push(TypedValue::Number(*val));
                }
                crate::vm::Op::Add => {
                    let (b, a) = self.pop_two("add")?;
                    match a.add(&b) {
                        Ok(result) => self.stack.push(result),
                        Err(e) => return Err(self.type_error_to_vm_error(e)),
                    }
                }
                // Remaining operations would follow a similar pattern
                _ => {
                    // For this prototype, we'll just have placeholder support for key operations
                    self.stack.push(TypedValue::Null);
                }
            }
        }

        Ok(())
    }

    /// Convert a typed value error to a VM error
    fn type_error_to_vm_error(&self, error: TypedValueError) -> crate::vm::VMError {
        match error {
            TypedValueError::TypeMismatch { expected, found } => {
                crate::vm::VMError::ParameterError(format!(
                    "Type mismatch: expected {}, found {}",
                    expected, found
                ))
            }
            TypedValueError::InvalidOperationForType { op, types } => {
                crate::vm::VMError::ParameterError(format!(
                    "Invalid operation {} for types {}",
                    op, types
                ))
            }
            TypedValueError::CoercionError { from, to } => {
                crate::vm::VMError::ParameterError(format!("Cannot coerce from {} to {}", from, to))
            }
            TypedValueError::ValueOutOfBounds => {
                crate::vm::VMError::ParameterError("Value out of bounds".to_string())
            }
        }
    }
}

impl Default for TypedVM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_basic_arithmetic() {
        // Addition
        let a = TypedValue::Number(5.0);
        let b = TypedValue::Number(3.0);
        assert_eq!(a.add(&b).unwrap(), TypedValue::Number(8.0));

        // String concatenation
        let a = TypedValue::String("Hello, ".to_string());
        let b = TypedValue::String("World!".to_string());
        assert_eq!(
            a.add(&b).unwrap(),
            TypedValue::String("Hello, World!".to_string())
        );

        // Mixed type addition
        let a = TypedValue::String("Count: ".to_string());
        let b = TypedValue::Number(42.0);
        assert_eq!(
            a.add(&b).unwrap(),
            TypedValue::String("Count: 42".to_string())
        );
    }

    #[test]
    fn test_typed_boolean_operations() {
        let t = TypedValue::Boolean(true);
        let f = TypedValue::Boolean(false);

        // Logical operations
        assert_eq!(t.logical_and(&t).unwrap(), TypedValue::Boolean(true));
        assert_eq!(t.logical_and(&f).unwrap(), TypedValue::Boolean(false));
        assert_eq!(f.logical_or(&t).unwrap(), TypedValue::Boolean(true));
        assert_eq!(f.logical_or(&f).unwrap(), TypedValue::Boolean(false));
        assert_eq!(t.logical_not().unwrap(), TypedValue::Boolean(false));
        assert_eq!(f.logical_not().unwrap(), TypedValue::Boolean(true));
    }

    #[test]
    fn test_typed_comparisons() {
        let a = TypedValue::Number(5.0);
        let b = TypedValue::Number(3.0);

        assert_eq!(a.greater_than(&b).unwrap(), TypedValue::Boolean(true));
        assert_eq!(a.less_than(&b).unwrap(), TypedValue::Boolean(false));

        let a = TypedValue::String("abc".to_string());
        let b = TypedValue::String("def".to_string());

        assert_eq!(a.less_than(&b).unwrap(), TypedValue::Boolean(true));
        assert_eq!(a.greater_than(&b).unwrap(), TypedValue::Boolean(false));
    }

    #[test]
    fn test_typed_equality() {
        // Same types
        assert_eq!(
            TypedValue::Number(5.0)
                .equals(&TypedValue::Number(5.0))
                .unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            TypedValue::String("abc".to_string())
                .equals(&TypedValue::String("abc".to_string()))
                .unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            TypedValue::Boolean(true)
                .equals(&TypedValue::Boolean(true))
                .unwrap(),
            TypedValue::Boolean(true)
        );

        // Different types
        assert_eq!(
            TypedValue::Number(1.0)
                .equals(&TypedValue::Boolean(true))
                .unwrap(),
            TypedValue::Boolean(true)
        );
        assert_eq!(
            TypedValue::Number(0.0)
                .equals(&TypedValue::Boolean(false))
                .unwrap(),
            TypedValue::Boolean(true)
        );
    }
}
