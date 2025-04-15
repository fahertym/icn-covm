//! VM Memory and scope management
//! 
//! This module provides memory operations and scope management for the VM.

use std::collections::HashMap;
use std::fmt;

use crate::vm::errors::VMError;
use crate::vm::types::CallFrame;

/// Provides memory operations for the virtual machine
#[derive(Debug, Clone)]
pub struct VMMemory {
    /// Global memory for storing variables
    memory: HashMap<String, f64>,
    
    /// Function map for storing subroutines (params, body)
    functions: HashMap<String, (Vec<String>, Vec<crate::vm::types::Op>)>,
    
    /// Call stack for tracking function calls
    call_stack: Vec<usize>,
    
    /// Call frames for function memory scoping
    call_frames: Vec<CallFrame>,
    
    /// Runtime parameters
    parameters: HashMap<String, String>,
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
        }
    }

    /// Store a value in global memory
    pub fn store(&mut self, name: &str, value: f64) {
        // If we're in a function call frame, store in local memory
        if !self.call_frames.is_empty() {
            if let Some(frame) = self.call_frames.last_mut() {
                frame.memory.insert(name.to_string(), value);
                return;
            }
        }
        
        // Otherwise store in global memory
        self.memory.insert(name.to_string(), value);
    }

    /// Load a value from memory, checking scopes from innermost to outermost
    pub fn load(&self, name: &str) -> Result<f64, VMError> {
        // Check function parameters first (if in a function)
        if !self.call_frames.is_empty() {
            if let Some(frame) = self.call_frames.last() {
                // Check parameters
                if let Some(value) = frame.params.get(name) {
                    return Ok(*value);
                }
                
                // Check local memory
                if let Some(value) = frame.memory.get(name) {
                    return Ok(*value);
                }
            }
        }
        
        // Fall back to global memory
        self.memory
            .get(name)
            .copied()
            .ok_or_else(|| VMError::VariableNotFound(name.to_string()))
    }

    /// Define a function in memory
    pub fn define_function(&mut self, name: &str, params: Vec<String>, body: Vec<crate::vm::types::Op>) {
        self.functions.insert(name.to_string(), (params, body));
    }

    /// Get a function by name
    pub fn get_function(&self, name: &str) -> Result<(Vec<String>, Vec<crate::vm::types::Op>), VMError> {
        self.functions
            .get(name)
            .cloned()
            .ok_or_else(|| VMError::FunctionNotFound(name.to_string()))
    }

    /// Push a new call frame onto the call stack
    pub fn push_call_frame(&mut self, function_name: &str, params: HashMap<String, f64>) -> usize {
        let frame = CallFrame {
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
    pub fn pop_call_frame(&mut self) -> Option<CallFrame> {
        self.call_stack.pop();
        self.call_frames.pop()
    }

    /// Get a reference to the current call frame
    pub fn current_call_frame(&self) -> Option<&CallFrame> {
        if self.call_frames.is_empty() {
            None
        } else {
            self.call_frames.last()
        }
    }

    /// Get a mutable reference to the current call frame
    pub fn current_call_frame_mut(&mut self) -> Option<&mut CallFrame> {
        if self.call_frames.is_empty() {
            None
        } else {
            self.call_frames.last_mut()
        }
    }

    /// Set the return value for the current call frame
    pub fn set_return_value(&mut self, value: f64) -> Result<(), VMError> {
        let frame = self.current_call_frame_mut()
            .ok_or_else(|| VMError::NotImplemented("Cannot return outside a function".to_string()))?;
        
        frame.return_value = Some(value);
        Ok(())
    }

    /// Get the return value from the current call frame
    pub fn get_return_value(&self) -> Option<f64> {
        self.current_call_frame().and_then(|frame| frame.return_value)
    }

    /// Set runtime parameters
    pub fn set_parameters(&mut self, parameters: HashMap<String, String>) {
        self.parameters = parameters;
    }

    /// Get a parameter by name
    pub fn get_parameter(&self, name: &str) -> Result<String, VMError> {
        self.parameters
            .get(name)
            .cloned()
            .ok_or_else(|| VMError::ParameterNotFound(name.to_string()))
    }

    /// Get a copy of the current memory map
    pub fn get_memory_map(&self) -> HashMap<String, f64> {
        self.memory.clone()
    }

    /// Format the memory as a string for display
    pub fn format_memory(&self) -> String {
        if self.memory.is_empty() {
            return "Memory: {}".to_string();
        }
        
        let mut result = "Memory: {\n".to_string();
        
        // Sort keys for consistent display
        let mut keys: Vec<&String> = self.memory.keys().collect();
        keys.sort();
        
        for key in keys {
            if let Some(value) = self.memory.get(key) {
                result.push_str(&format!("  {}: {}\n", key, value));
            }
        }
        
        result.push_str("}");
        result
    }

    /// Format the call stack as a string for display
    pub fn format_call_stack(&self) -> String {
        if self.call_frames.is_empty() {
            return "Call Stack: []".to_string();
        }
        
        let mut result = "Call Stack: [\n".to_string();
        
        for (i, frame_idx) in self.call_stack.iter().enumerate() {
            if let Some(frame) = self.call_frames.get(*frame_idx) {
                result.push_str(&format!("  {}. {}\n", i, frame.function_name));
            }
        }
        
        result.push_str("]");
        result
    }

    /// Clear all global memory
    pub fn clear_memory(&mut self) {
        self.memory.clear();
    }

    /// Check if we're currently in a function call
    pub fn in_function_call(&self) -> bool {
        !self.call_frames.is_empty()
    }

    /// Get call stack depth
    pub fn call_stack_depth(&self) -> usize {
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
    use crate::vm::types::Op;

    #[test]
    fn test_store_load() {
        let mut memory = VMMemory::new();
        memory.store("x", 42.0);
        assert_eq!(memory.load("x").unwrap(), 42.0);
    }

    #[test]
    fn test_variable_not_found() {
        let memory = VMMemory::new();
        let result = memory.load("nonexistent");
        assert!(matches!(result, Err(VMError::VariableNotFound(_))));
    }

    #[test]
    fn test_function_definition() {
        let mut memory = VMMemory::new();
        let params = vec!["a".to_string(), "b".to_string()];
        let body = vec![Op::Add, Op::Return];
        
        memory.define_function("add", params.clone(), body.clone());
        
        let (retrieved_params, retrieved_body) = memory.get_function("add").unwrap();
        assert_eq!(retrieved_params, params);
        assert_eq!(retrieved_body, body);
    }

    #[test]
    fn test_scoped_memory() {
        let mut memory = VMMemory::new();
        
        // Store in global scope
        memory.store("global", 100.0);
        
        // Create function call frame with parameters
        let mut params = HashMap::new();
        params.insert("param".to_string(), 200.0);
        memory.push_call_frame("test_function", params);
        
        // Store in local scope
        memory.store("local", 300.0);
        
        // All variables should be accessible in inner scope
        assert_eq!(memory.load("global").unwrap(), 100.0);
        assert_eq!(memory.load("param").unwrap(), 200.0);
        assert_eq!(memory.load("local").unwrap(), 300.0);
        
        // Pop the call frame
        memory.pop_call_frame();
        
        // Local and param should no longer be accessible
        assert_eq!(memory.load("global").unwrap(), 100.0);
        assert!(matches!(memory.load("local"), Err(VMError::VariableNotFound(_))));
        assert!(matches!(memory.load("param"), Err(VMError::VariableNotFound(_))));
    }
} 