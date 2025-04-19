//! VM Memory operations
//!
//! This module provides memory manipulation operations for the VM.
//!
//! The memory system is responsible for:
//! - Storing and retrieving named variables
//! - Managing function definitions and calls
//! - Supporting memory scopes for different execution contexts
//!
//! The module defines a `MemoryScope` trait that encapsulates the operations
//! that can be performed on memory, enabling alternative memory implementations
//! if needed.

use crate::typed::TypedValue;
use crate::vm::errors::VMError;
use crate::vm::types::{CallFrame, Op};
use std::collections::HashMap;
use std::fmt;

/// Call frame for function scope
#[derive(Debug, Clone)]
pub struct TypedCallFrame {
    /// Local memory for this function call
    pub memory: HashMap<String, TypedValue>,
    
    /// Parameters passed to this function
    pub params: HashMap<String, TypedValue>,
    
    /// Return value if set
    pub return_value: Option<TypedValue>,
    
    /// Name of the function being called
    pub function_name: String,
}

/// Defines operations for memory scope
pub trait MemoryScope {
    /// Store a value in memory
    fn store(&mut self, name: &str, value: TypedValue);

    /// Load a value from memory
    fn load(&self, name: &str) -> Result<TypedValue, VMError>;

    /// Define a function in memory
    fn define_function(&mut self, name: &str, params: Vec<String>, body: Vec<Op>);

    /// Get a function by name
    fn get_function(&self, name: &str) -> Result<(Vec<String>, Vec<Op>), VMError>;

    /// Push a new call frame onto the call stack
    fn push_call_frame(&mut self, function_name: &str, params: HashMap<String, TypedValue>) -> usize;

    /// Pop the current call frame
    fn pop_call_frame(&mut self) -> Option<TypedCallFrame>;

    /// Get a reference to the current call frame
    fn current_call_frame(&self) -> Option<&TypedCallFrame>;

    /// Get a mutable reference to the current call frame
    fn current_call_frame_mut(&mut self) -> Option<&mut TypedCallFrame>;

    /// Set the return value for the current call frame
    fn set_return_value(&mut self, value: TypedValue) -> Result<(), VMError>;

    /// Get the return value from the current call frame
    fn get_return_value(&self) -> Option<TypedValue>;

    /// Set runtime parameters
    fn set_parameters(&mut self, parameters: HashMap<String, String>);

    /// Get a parameter by name
    fn get_parameter(&self, name: &str) -> Result<String, VMError>;

    /// Get a copy of the current memory map
    fn get_memory_map(&self) -> HashMap<String, TypedValue>;

    /// Format the memory as a string for display
    fn format_memory(&self) -> String;

    /// Format the call stack as a string for display
    fn format_call_stack(&self) -> String;

    /// Clear all global memory
    fn clear_memory(&mut self);

    /// Check if we're currently in a function call
    fn in_function_call(&self) -> bool;

    /// Get call stack depth
    fn call_stack_depth(&self) -> usize;
}

/// Provides memory operations for the virtual machine
#[derive(Debug, Clone)]
pub struct VMMemory {
    /// Global memory for storing variables
    memory: HashMap<String, TypedValue>,

    /// Function map for storing subroutines (params, body)
    functions: HashMap<String, (Vec<String>, Vec<Op>)>,

    /// Call stack for tracking function calls
    call_stack: Vec<usize>,

    /// Call frames for function memory scoping
    call_frames: Vec<TypedCallFrame>,

    /// Runtime parameters
    parameters: HashMap<String, String>,

    /// String metadata for extra storage needs (JSON, etc.)
    string_metadata: HashMap<String, String>,
}

impl VMMemory {
    /// Create a new empty memory space
    pub fn new() -> Self {
        Self {
            memory: HashMap::new(),
            functions: HashMap::new(),
            call_stack: Vec::new(),
            call_frames: Vec::new(),
            parameters: HashMap::new(),
            string_metadata: HashMap::new(),
        }
    }

    /// Get string metadata for a key
    pub fn get_string_metadata(&self, key: &str) -> Option<String> {
        self.string_metadata.get(key).cloned()
    }

    /// Set string metadata for a key
    pub fn set_string_metadata(&mut self, key: &str, value: String) {
        self.string_metadata.insert(key.to_string(), value);
    }
}

impl MemoryScope for VMMemory {
    /// Store a value in memory
    fn store(&mut self, name: &str, value: TypedValue) {
        if let Some(frame_idx) = self.call_stack.last() {
            // Store in the current call frame
            let frame = &mut self.call_frames[*frame_idx];
            frame.memory.insert(name.to_string(), value);
        } else {
            // Store in global memory
            self.memory.insert(name.to_string(), value);
        }
    }

    /// Load a value from memory
    fn load(&self, name: &str) -> Result<TypedValue, VMError> {
        // First check current call frame
        if let Some(frame_idx) = self.call_stack.last() {
            let frame = &self.call_frames[*frame_idx];
            
            // Check local memory first
            if let Some(value) = frame.memory.get(name) {
                return Ok(value.clone());
            }
            
            // Check params
            if let Some(value) = frame.params.get(name) {
                return Ok(value.clone());
            }
        }
        
        // Check global memory
        self.memory.get(name).cloned().ok_or_else(|| VMError::UndefinedVariable {
            name: name.to_string(),
        })
    }

    /// Define a function in memory
    fn define_function(&mut self, name: &str, params: Vec<String>, body: Vec<Op>) {
        self.functions.insert(name.to_string(), (params, body));
    }

    /// Get a function by name
    fn get_function(&self, name: &str) -> Result<(Vec<String>, Vec<Op>), VMError> {
        self.functions
            .get(name)
            .cloned()
            .ok_or_else(|| VMError::UndefinedFunction {
                name: name.to_string(),
            })
    }

    /// Push a new call frame onto the call stack
    fn push_call_frame(&mut self, function_name: &str, params: HashMap<String, TypedValue>) -> usize {
        let frame = TypedCallFrame {
            memory: HashMap::new(),
            params,
            return_value: None,
            function_name: function_name.to_string(),
        };

        self.call_frames.push(frame);
        self.call_stack.push(self.call_frames.len() - 1);
        self.call_frames.len() - 1
    }

    /// Pop the current call frame
    fn pop_call_frame(&mut self) -> Option<TypedCallFrame> {
        self.call_stack.pop();
        self.call_frames.pop()
    }

    /// Get a reference to the current call frame
    fn current_call_frame(&self) -> Option<&TypedCallFrame> {
        if self.call_frames.is_empty() {
            None
        } else {
            self.call_frames.last()
        }
    }

    /// Get a mutable reference to the current call frame
    fn current_call_frame_mut(&mut self) -> Option<&mut TypedCallFrame> {
        if self.call_frames.is_empty() {
            None
        } else {
            self.call_frames.last_mut()
        }
    }

    /// Set the return value for the current call frame
    fn set_return_value(&mut self, value: TypedValue) -> Result<(), VMError> {
        let frame = self.current_call_frame_mut().ok_or_else(|| {
            VMError::NotImplemented("Cannot return outside a function".to_string())
        })?;

        frame.return_value = Some(value);
        Ok(())
    }

    /// Get the return value from the current call frame
    fn get_return_value(&self) -> Option<TypedValue> {
        self.current_call_frame()
            .and_then(|frame| frame.return_value.clone())
    }

    /// Set runtime parameters
    fn set_parameters(&mut self, parameters: HashMap<String, String>) {
        self.parameters = parameters;
        
        // Also convert parameters to typed values in memory
        for (key, value) in &self.parameters {
            // Try to parse as number first
            if let Ok(num) = value.parse::<f64>() {
                self.memory.insert(key.clone(), TypedValue::Number(num));
            } else if value == "true" {
                self.memory.insert(key.clone(), TypedValue::Boolean(true));
            } else if value == "false" {
                self.memory.insert(key.clone(), TypedValue::Boolean(false));
            } else if value == "null" {
                self.memory.insert(key.clone(), TypedValue::Null);
            } else {
                // Store as string
                self.memory.insert(key.clone(), TypedValue::String(value.clone()));
            }
        }
    }

    /// Get a parameter by name
    fn get_parameter(&self, name: &str) -> Result<String, VMError> {
        self.parameters.get(name).cloned().ok_or_else(|| VMError::UndefinedParameter {
            name: name.to_string(),
        })
    }

    /// Get a copy of the current memory map
    fn get_memory_map(&self) -> HashMap<String, TypedValue> {
        if let Some(frame_idx) = self.call_stack.last() {
            let frame = &self.call_frames[*frame_idx];
            let mut merged = self.memory.clone();
            
            // Add params
            for (k, v) in &frame.params {
                merged.insert(k.clone(), v.clone());
            }
            
            // Add local memory (overriding global if needed)
            for (k, v) in &frame.memory {
                merged.insert(k.clone(), v.clone());
            }
            
            merged
        } else {
            self.memory.clone()
        }
    }

    /// Format the memory as a string for display
    fn format_memory(&self) -> String {
        let mem_map = self.get_memory_map();
        if mem_map.is_empty() {
            return "Memory: {}".to_string();
        }

        let mut items = Vec::new();
        for (k, v) in &mem_map {
            items.push(format!("{}: {}", k, v));
        }
        
        items.sort();
        format!("Memory: {{\n  {}\n}}", items.join(",\n  "))
    }

    /// Format the call stack as a string for display
    fn format_call_stack(&self) -> String {
        if self.call_frames.is_empty() {
            return "Call Stack: []".to_string();
        }

        let mut items = Vec::new();
        for (i, frame_idx) in self.call_stack.iter().enumerate() {
            let frame = &self.call_frames[*frame_idx];
            items.push(format!("{}. {}", i, frame.function_name));
        }
        format!("Call Stack: [{}]", items.join(", "))
    }

    /// Clear all global memory
    fn clear_memory(&mut self) {
        self.memory.clear();
    }

    /// Check if we're currently in a function call
    fn in_function_call(&self) -> bool {
        !self.call_stack.is_empty()
    }

    /// Get call stack depth
    fn call_stack_depth(&self) -> usize {
        self.call_stack.len()
    }
}

impl fmt::Display for VMMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.format_memory())?;
        writeln!(f, "{}", self.format_call_stack())?;

        if !self.functions.is_empty() {
            writeln!(f, "Functions: [")?;

            for name in self.functions.keys() {
                writeln!(f, "  {}", name)?;
            }

            writeln!(f, "]")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_store_load() {
        let mut memory = VMMemory::new();
        memory.store("x", TypedValue::Number(42.0));
        
        assert_eq!(memory.load("x").unwrap(), TypedValue::Number(42.0));
    }

    #[test]
    fn test_memory_typed_values() {
        let mut memory = VMMemory::new();
        
        // Store different types
        memory.store("num", TypedValue::Number(42.0));
        memory.store("bool", TypedValue::Boolean(true));
        memory.store("str", TypedValue::String("hello".to_string()));
        memory.store("null", TypedValue::Null);
        
        // Verify retrieval
        assert_eq!(memory.load("num").unwrap(), TypedValue::Number(42.0));
        assert_eq!(memory.load("bool").unwrap(), TypedValue::Boolean(true));
        assert_eq!(memory.load("str").unwrap(), TypedValue::String("hello".to_string()));
        assert_eq!(memory.load("null").unwrap(), TypedValue::Null);
    }

    #[test]
    fn test_call_frame_scoping() {
        let mut memory = VMMemory::new();
        
        // Set global variable
        memory.store("x", TypedValue::Number(1.0));
        
        // Create a call frame
        let mut params = HashMap::new();
        params.insert("y".to_string(), TypedValue::Number(2.0));
        memory.push_call_frame("test_function", params);
        
        // Set local variable
        memory.store("z", TypedValue::Number(3.0));
        
        // Local scope should have access to all variables
        assert_eq!(memory.load("x").unwrap(), TypedValue::Number(1.0));
        assert_eq!(memory.load("y").unwrap(), TypedValue::Number(2.0));
        assert_eq!(memory.load("z").unwrap(), TypedValue::Number(3.0));
        
        // Return from function
        memory.pop_call_frame();
        
        // Global scope should only have x
        assert_eq!(memory.load("x").unwrap(), TypedValue::Number(1.0));
        assert!(memory.load("y").is_err());
        assert!(memory.load("z").is_err());
    }
}
