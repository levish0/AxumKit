use crate::config::db_config::DbConfig;
use redis::aio::ConnectionManager;
use redis::{Client, RedisResult};
use tracing::info;

pub async fn establish_redis_connection() -> RedisResult<ConnectionManager> {
    let redis_url = format!(
        "redis://{}:{}",
        &DbConfig::get().redis_host,
        &DbConfig::get().redis_port,
    );
    info!("Connecting to Redis at: {}", redis_url);

    let client = Client::open(redis_url.as_str())?;
    let conn_manager = ConnectionManager::new(client).await?;

    info!("Successfully connected to Redis");
    Ok(conn_manager)
}