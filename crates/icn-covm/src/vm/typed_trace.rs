//! VM Execution Trace and Debug Utilities
//!
//! This module provides tracing and debugging utilities for the VM execution,
//! enabling better visibility into program execution and error diagnosis.

use crate::typed::TypedValue;
use crate::vm::VMStack;
use crate::vm::types::{Op, VMEvent};
use crate::vm::stack::StackOps;

/// Represents a single frame in the VM execution trace
#[derive(Debug, Clone)]
pub struct TypedFrameTrace {
    /// Operation being executed
    pub op: Op,
    
    /// Stack state before execution
    pub stack_before: Vec<TypedValue>,
    
    /// Stack state after execution
    pub stack_after: Option<Vec<TypedValue>>,
    
    /// Memory variables modified in this frame
    pub memory_changes: Vec<(String, TypedValue)>,
    
    /// Events emitted during this frame
    pub events_emitted: Vec<VMEvent>,
    
    /// Program counter
    pub pc: usize,
    
    /// If an error occurred during this frame
    pub error: Option<String>,
}

impl TypedFrameTrace {
    /// Create a new execution trace frame
    pub fn new(op: &Op, stack: &VMStack, pc: usize) -> Self {
        Self {
            op: op.clone(),
            stack_before: stack.get_stack(),
            stack_after: None,
            memory_changes: Vec::new(),
            events_emitted: Vec::new(),
            pc,
            error: None,
        }
    }
    
    /// Record the stack state after execution
    pub fn record_stack_after(&mut self, stack: &VMStack) {
        self.stack_after = Some(stack.get_stack());
    }
    
    /// Record a memory change
    pub fn record_memory_change(&mut self, name: &str, value: &TypedValue) {
        self.memory_changes.push((name.to_string(), value.clone()));
    }
    
    /// Record emitted events
    pub fn record_events(&mut self, events: &[VMEvent]) {
        self.events_emitted.extend(events.iter().cloned());
    }
    
    /// Record an error
    pub fn record_error(&mut self, error: String) {
        self.error = Some(error);
    }
    
    /// Format as an explanation string
    pub fn explain(&self) -> String {
        let mut explanation = format!("Step {}: {}\n", self.pc, self.op_description());
        
        // Stack before
        explanation.push_str("  Stack before: [");
        explanation.push_str(&self.format_stack(&self.stack_before));
        explanation.push_str("]\n");
        
        // Stack after (if available)
        if let Some(stack_after) = &self.stack_after {
            explanation.push_str("  Stack after: [");
            explanation.push_str(&self.format_stack(stack_after));
            explanation.push_str("]\n");
        }
        
        // Memory changes
        if !self.memory_changes.is_empty() {
            explanation.push_str("  Memory changes:\n");
            for (name, value) in &self.memory_changes {
                explanation.push_str(&format!("    {} = {}\n", name, value.describe()));
            }
        }
        
        // Events
        if !self.events_emitted.is_empty() {
            explanation.push_str("  Events:\n");
            for event in &self.events_emitted {
                explanation.push_str(&format!("    [{}] {}\n", event.category, event.message));
            }
        }
        
        // Error
        if let Some(error) = &self.error {
            explanation.push_str(&format!("  ERROR: {}\n", error));
        }
        
        explanation
    }
    
    /// Get a descriptive name for the operation
    fn op_description(&self) -> String {
        match &self.op {
            Op::Push(val) => format!("Push {}", val.describe()),
            Op::Pop => "Pop value from stack".to_string(),
            Op::Add => "Add top two values".to_string(),
            Op::Sub => "Subtract top value from second value".to_string(),
            Op::Mul => "Multiply top two values".to_string(),
            Op::Div => "Divide second value by top value".to_string(),
            Op::Mod => "Modulo second value by top value".to_string(),
            Op::Store(name) => format!("Store value in variable '{}'", name),
            Op::Load(name) => format!("Load variable '{}' onto stack", name),
            Op::Eq => "Check if top two values are equal".to_string(),
            Op::Lt => "Check if second value is less than top value".to_string(),
            Op::Gt => "Check if second value is greater than top value".to_string(),
            Op::And => "Logical AND of top two values".to_string(),
            Op::Or => "Logical OR of top two values".to_string(),
            Op::Not => "Logical NOT of top value".to_string(),
            Op::If { .. } => "Conditional block".to_string(),
            Op::Loop { .. } => "Loop block".to_string(),
            Op::Call(name) => format!("Call function '{}'", name),
            _ => format!("{:?}", self.op),  // Fallback for other ops
        }
    }
    
    /// Format a stack for display
    fn format_stack(&self, stack: &[TypedValue]) -> String {
        stack.iter()
            .map(|v| v.describe())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Simplified trace frame for external use
#[derive(Debug, Clone)]
pub struct TypedTraceFrame {
    /// Operation being executed
    pub op: Op,
    /// Stack state before execution
    pub stack_before: Vec<TypedValue>,
    /// Stack state after execution
    pub stack_after: Vec<TypedValue>,
}

/// Execution tracer that records and displays VM execution
#[derive(Debug, Default)]
pub struct VMTracer {
    /// Complete execution trace
    pub frames: Vec<TypedFrameTrace>,
    
    /// Whether tracing is enabled
    pub enabled: bool,
    
    /// Verbosity level
    pub verbosity: usize,
    
    /// External trace frames (simplified)
    pub external_frames: Vec<TypedTraceFrame>,
}

impl VMTracer {
    /// Create a new VM tracer
    pub fn new(enabled: bool, verbosity: usize) -> Self {
        Self {
            frames: Vec::new(),
            enabled,
            verbosity,
            external_frames: Vec::new(),
        }
    }
    
    /// Start tracing a new operation
    pub fn trace_op(&mut self, op: &Op, stack: &VMStack, pc: usize) -> Option<usize> {
        if !self.enabled {
            return None;
        }
        
        let frame = TypedFrameTrace::new(op, stack, pc);
        self.frames.push(frame);
        Some(self.frames.len() - 1)
    }
    
    /// Update the stack state after execution
    pub fn trace_stack_after(&mut self, frame_idx: Option<usize>, stack: &VMStack) {
        if let Some(idx) = frame_idx {
            if let Some(frame) = self.frames.get_mut(idx) {
                frame.record_stack_after(stack);
            }
        }
    }
    
    /// Record a memory change
    pub fn trace_memory_change(&mut self, frame_idx: Option<usize>, name: &str, value: &TypedValue) {
        if let Some(idx) = frame_idx {
            if let Some(frame) = self.frames.get_mut(idx) {
                frame.record_memory_change(name, value);
            }
        }
    }
    
    /// Record emitted events
    pub fn trace_events(&mut self, frame_idx: Option<usize>, events: &[VMEvent]) {
        if let Some(idx) = frame_idx {
            if let Some(frame) = self.frames.get_mut(idx) {
                frame.record_events(events);
            }
        }
    }
    
    /// Record an error
    pub fn trace_error(&mut self, frame_idx: Option<usize>, error: String) {
        if let Some(idx) = frame_idx {
            if let Some(frame) = self.frames.get_mut(idx) {
                frame.record_error(error);
            }
        }
    }
    
    /// Record a simplified trace frame for external use
    pub fn record_trace_frame(&mut self, op: Op, stack_before: Vec<TypedValue>, stack_after: Vec<TypedValue>) {
        if self.enabled {
            self.external_frames.push(TypedTraceFrame {
                op,
                stack_before,
                stack_after,
            });
        }
    }
    
    /// Generate an execution report
    pub fn generate_report(&self) -> String {
        if !self.enabled || self.frames.is_empty() {
            return "Tracing not enabled or no execution frames recorded.".to_string();
        }
        
        let mut report = String::from("=== VM Execution Trace ===\n\n");
        
        for frame in &self.frames {
            report.push_str(&frame.explain());
            report.push_str("\n");
        }
        
        report.push_str("=== End of Trace ===\n");
        report
    }
    
    /// Print the execution report
    pub fn print_report(&self) {
        println!("{}", self.generate_report());
    }
}

/// Implementation to add VM tracing capability to VM
/// This trait can be implemented by the VM struct to add tracing support
pub trait TracedExecution {
    /// Enable or disable tracing
    fn set_tracing(&mut self, enabled: bool);
    
    /// Set the tracing verbosity level
    fn set_trace_verbosity(&mut self, level: usize);
    
    /// Get the execution trace
    fn get_trace(&self) -> Option<&VMTracer>;
    
    /// Print the execution trace
    fn print_trace(&self);
    
    /// Record a trace frame
    fn record_frame(&mut self, op: Op, stack_before: Vec<TypedValue>, stack_after: Vec<TypedValue>);
} 