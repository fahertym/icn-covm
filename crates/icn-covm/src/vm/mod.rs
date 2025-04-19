//! Virtual Machine for Cooperative Value Network Operations
//!
//! This module contains the implementation of the VM that executes operations
//! for the cooperative value network. The VM is designed with a modular architecture
//! to improve maintainability, testability, and extensibility.
//!
//! ## Modular Architecture
//!
//! The VM is divided into several focused components, each with a clear responsibility:
//!
//! - **stack.rs**: Manages the execution stack that provides push/pop operations and value manipulation.
//!   Defined by the `StackOps` trait and implemented by `VMStack`.
//!
//! - **memory.rs**: Handles variable storage, function definitions, scope management and call frames.
//!   Defined by the `MemoryScope` trait and implemented by `VMMemory`.
//!
//! - **execution.rs**: Implements operation execution logic, including storage interactions and
//!   transaction management. Defined by the `ExecutorOps` trait and implemented by `VMExecution`.
//!
//! - **types.rs**: Defines core data structures like operations (`Op`), call frames, and events.
//!
//! - **errors.rs**: Centralizes error handling for all VM operations.
//!
//! - **vm.rs**: Orchestrates the components, providing the main execution loop and API.
//!
//! - **typed_trace.rs**: Provides utilities for tracing and debugging VM execution.
//!
//! ## Benefits of Modular Design
//!
//! This modular design provides significant benefits:
//!
//! 1. **Separation of Concerns**: Each component focuses on a specific responsibility
//! 2. **Independent Testing**: Components can be tested independently
//! 3. **Extensibility**: New features can be added with minimal impact on other components
//! 4. **Maintainability**: Code organization follows logical boundaries
//! 5. **Performance Optimization**: Components can be optimized independently
//! 6. **Alternative Implementations**: Each trait can have multiple implementations
//!
//! ## Future Extensibility
//!
//! The trait-based design enables future extensions such as:
//!
//! - Alternative storage backends through the `Storage` trait
//! - Different execution strategies via the `ExecutorOps` trait
//! - Memory optimization through alternative `MemoryScope` implementations
//! - Potential for WASM integration or other execution environments
//! - Metered execution for resource usage tracking
//!
//! For more detailed information, see the documentation for each component.

// Module declarations
pub mod errors;
pub mod execution;
pub mod memory;
pub mod ops;
pub mod stack;
pub mod types;
mod vm;
pub mod typed_trace;

// Re-export main VM types and components
pub use errors::VMError;
pub use execution::{ExecutorOps, VMExecution};
pub use memory::{MemoryScope, VMMemory};
pub use stack::{StackOps, VMStack};
pub use types::{CallFrame, LoopControl, Op, VMEvent};
pub use vm::VM;
pub use typed_trace::{TypedFrameTrace, TypedTraceFrame, VMTracer, TracedExecution};

// Tests are kept in the vm.rs file for now
#[cfg(test)]
pub mod tests {
    pub use crate::vm::vm::tests;
}

pub use self::memory::Memory;
pub use self::stack::Stack;

/// Behavior when a key is not found in storage
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MissingKeyBehavior {
    /// Return a default value (0.0) when a key is not found
    Default,
    
    /// Return an error when a key is not found
    Error,
}
