use crate::CacheClient;
use crate::config::WorkerConfig;
use std::sync::Arc;
use tracing::info;

pub async fn establish_redis_cache_connection(
    config: &WorkerConfig,
) -> anyhow::Result<CacheClient> {
    let redis_cache_url = config.redis_cache_url();
    info!(url = %redis_cache_url, "Connecting to Redis cache");

    let redis_cache_client = redis::Client::open(redis_cache_url)?;
    let redis_cache_conn = redis::aio::ConnectionManager::new(redis_cache_client).await?;

    info!("Successfully connected to Redis cache");
    Ok(Arc::new(redis_cache_conn))
}
