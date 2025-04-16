use crate::vm::{VM, VMError};
use crate::vm::types::Op;
use std::fmt::Debug;
use std::marker::{Send, Sync};
use crate::storage::traits::Storage;

/// Trait for handling governance operations
pub trait GovernanceOpHandler {
    /// Handle a governance operation
    fn handle<S>(vm: &mut VM<S>, op: &Op) -> Result<(), VMError>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static;
} 