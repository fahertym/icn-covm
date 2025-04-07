pub mod bytecode;
pub mod compiler;
pub mod events;
pub mod storage;
pub mod vm;

// Use specific imports rather than assuming re-exports for clarity
pub use crate::compiler::{parse_dsl, parse_dsl_with_stdlib, CompilerError, SourcePosition};
pub use crate::events::{set_log_file, set_log_format, Event, LogFormat};
pub use crate::storage::errors::{StorageError, StorageResult};
pub use crate::storage::traits::StorageBackend;
pub use crate::storage::implementations::in_memory::InMemoryStorage;
// pub use crate::storage::implementations::file_storage::FileStorage; // FileStorage doesn't exist yet
pub use crate::vm::{Op, VMError, VM};
