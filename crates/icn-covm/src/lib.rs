//! Cooperative Value Module for the Internet Computer Network
//!
//! The `icn-covm` crate provides a virtual machine for executing
//! cooperative governance operations on the Internet Computer Network.
//!
//! Key features:
//! - Stack-based VM with rich operation types
//! - Serializable operations for storage and transmission
//! - DSL for writing governance programs
//! - Compiler for transforming DSL into VM operations
//! - Runtime for executing operations
//! - Storage abstractions for persistence
//!
//! This crate is intended to be used in contexts where multiple parties
//! need to cooperatively manage resources using programmatic governance.

pub mod bytecode;
pub mod compiler;
pub mod federation;
pub mod governance;
pub mod identity;
pub mod storage;
pub mod typed;
pub mod vm;

// Re-export key types for convenience
pub use typed::TypedValue;
pub use vm::types::Op;
pub use vm::VM;
