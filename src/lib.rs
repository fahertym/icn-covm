pub mod bytecode;
pub mod cli;
pub mod compiler;
pub mod events;
pub mod federation;
pub mod governance;
pub mod identity;
pub mod storage;
pub mod vm;

#[cfg(feature = "typed-values")]
pub mod typed;

// Use specific imports rather than assuming re-exports for clarity
pub use crate::compiler::{parse_dsl, parse_dsl_with_stdlib, CompilerError, SourcePosition};
pub use crate::events::{set_log_file, set_log_format, Event, LogFormat};
pub use crate::federation::{
    FederationError, NetworkEvent, NetworkMessage, NetworkNode, NodeConfig,
};
pub use crate::identity::{Identity, IdentityError, Profile};
pub use crate::storage::errors::{StorageError, StorageResult};
pub use crate::storage::implementations::file_storage::FileStorage;
pub use crate::storage::implementations::in_memory::InMemoryStorage;
pub use crate::storage::namespaces::{NamespaceMetadata, NamespaceRegistry};
pub use crate::storage::traits::{StorageBackend, StorageExtensions};
pub use crate::vm::{Op, VMError, VM};
