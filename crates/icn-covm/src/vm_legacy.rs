//! Virtual Machine for Cooperative Value Networks
//!
//! This module re-exports the VM implementation from the vm/ module.
//! It is maintained for backwards compatibility with existing code.

// Re-export modular VM structure
pub use crate::vm::errors::VMError;
pub use crate::vm::types::{Op, VMEvent};
pub use crate::vm::VM;

// The rest of the vm.rs implementation has been refactored into submodules
// under the `vm/` directory. See there for the full implementation.
