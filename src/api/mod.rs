pub mod proposal_api;

use crate::storage::traits::{Storage, StorageExtensions};
use crate::vm::VM;
use std::fmt::Debug;

/// Initializes and runs the HTTP API server
pub async fn start_api_server<S>(vm: VM<S>, port: u16) -> Result<(), Box<dyn std::error::Error>>
where
    S: Storage + StorageExtensions + Send + Sync + Clone + Debug + 'static,
{
    proposal_api::start_api(vm, port).await
}
