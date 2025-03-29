pub mod vm;
pub mod compiler;
pub mod events;
pub use vm::{Op, VM, VMError};
pub use compiler::{parse_dsl, parse_dsl_with_stdlib, CompilerError, SourcePosition};
pub use events::{Event, LogFormat, set_log_format, set_log_file}; 