use crate::config::server_config::ServerConfig;
use redis::aio::ConnectionManager;
use redis::{Client, RedisResult};
use tracing::info;

pub async fn establish_redis_connection() -> RedisResult<ConnectionManager> {
    let redis_url = format!(
        "redis://{}:{}",
        &ServerConfig::get().redis_host,
        &ServerConfig::get().redis_port,
    );
    info!("Connecting to Redis at: {}", redis_url);

    let client = Client::open(redis_url.as_str())?;
    let conn_manager = ConnectionManager::new(client).await?;

    info!("Successfully connected to Redis");
    Ok(conn_manager)
}
