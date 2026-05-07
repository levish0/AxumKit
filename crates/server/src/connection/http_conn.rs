use reqwest::Client;
use std::time::Duration;
use tracing::{error, info};

pub async fn create_http_client() -> Result<Client, reqwest::Error> {
    info!("Creating HTTP client");

    let client = Client::builder()
        .timeout(Duration::from_secs(30)) // Overall request timeout
        .connect_timeout(Duration::from_secs(10)) // Connection timeout
        .pool_idle_timeout(Duration::from_secs(90)) // Idle connection timeout
        .pool_max_idle_per_host(10) // Max idle connections per host
        .user_agent("axumkit-server/1.0") // User-Agent configuration
        .tcp_keepalive(Duration::from_secs(60)) // TCP keep-alive
        .build()
        .map_err(|e| {
            error!("Failed to create HTTP client: {:?}", e);
            e
        })?;

    info!("Successfully created HTTP client");
    Ok(client)
}
