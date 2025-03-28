pub mod vm;
pub mod compiler;
pub use vm::{Op, VM};
pub use compiler::parse_dsl; 