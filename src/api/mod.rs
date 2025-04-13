pub mod proposal_api;

/// Initializes and runs the HTTP API server
pub async fn start_api_server<S>(
    vm: S,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: Send + Sync + Clone + 'static,
{
    proposal_api::start_api(vm, port).await
} 