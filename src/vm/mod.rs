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

// Re-export main VM types for backward compatibility
pub mod errors;
pub mod stack;
pub mod memory;
pub mod execution;
pub mod types;

pub use errors::VMError;
pub use types::{Op, CallFrame, LoopControl, VMEvent};
pub use stack::VMStack;
pub use memory::VMMemory;
pub use execution::VMExecution;

// Re-export the traits for public use
pub use stack::StackOps;
pub use memory::MemoryScope;
pub use execution::ExecutorOps;

// Main VM struct that coordinates components
mod vm;
pub use vm::VM;

// Tests are kept in the vm.rs file for now
#[cfg(test)]
mod tests {
    pub use crate::vm::vm::tests::*;
} 