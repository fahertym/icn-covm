//! Virtual Machine for Cooperative Value Network Operations
//! 
//! This module contains the implementation of the VM that executes operations
//! for the cooperative value network.

// Re-export main VM types for backward compatibility
mod errors;
mod stack;
mod memory;
mod execution;
mod types;

pub use errors::VMError;
pub use types::{Op, CallFrame, LoopControl, VMEvent};
pub use stack::VMStack;
pub use memory::VMMemory;
pub use execution::VMExecution;

// Main VM struct that coordinates components
mod vm;
pub use vm::VM;

// Tests are kept in the vm.rs file for now
#[cfg(test)]
mod tests {
    pub use crate::vm::vm::tests::*;
} 