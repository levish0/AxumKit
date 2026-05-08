use crate::LockClient;
use crate::config::WorkerConfig;
use std::sync::Arc;
use tracing::info;

pub async fn establish_redis_lock_connection(
    config: &WorkerConfig,
) -> anyhow::Result<LockClient> {
    let redis_lock_url = config.redis_session_url();
    info!(url = %redis_lock_url, "Connecting to Redis lock store");

    let redis_lock_client = redis::Client::open(redis_lock_url)?;
    let redis_lock_conn = redis::aio::ConnectionManager::new(redis_lock_client).await?;

    info!("Successfully connected to Redis lock store");
    Ok(Arc::new(redis_lock_conn))
}
