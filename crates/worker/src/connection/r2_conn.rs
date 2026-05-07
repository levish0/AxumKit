use crate::config::WorkerConfig;
use storage::{R2AssetsClient, R2Config, create_r2_client};
use tracing::info;

pub type R2Client = R2AssetsClient;

pub async fn establish_r2_connection(config: &WorkerConfig) -> anyhow::Result<R2Client> {
    info!(
        "Connecting to R2 at: {} (region: {})",
        config.r2_endpoint, config.r2_region
    );

    let client = create_r2_client(&R2Config {
        endpoint: config.r2_endpoint.clone(),
        region: config.r2_region.clone(),
        access_key_id: config.r2_access_key_id.clone(),
        secret_access_key: config.r2_secret_access_key.clone(),
    })
    .await;

    let r2_client = R2Client::new(
        client,
        config.r2_assets_bucket_name.clone(),
        config.r2_assets_public_domain.clone(),
    );

    info!("Successfully connected to R2");
    Ok(r2_client)
}
