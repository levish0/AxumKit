use redis::aio::ConnectionManager;
use redis::{Client, RedisResult};
use tracing::info;

pub async fn establish_redis_connection(
    host: &str,
    port: &str,
    name: &str,
) -> RedisResult<ConnectionManager> {
    let redis_url = format!("redis://{}:{}", host, port);
    info!("Connecting to Redis {} at: {}", name, redis_url);

    let client = Client::open(redis_url.as_str())?;
    let conn_manager = ConnectionManager::new(client).await?;

    info!("Successfully connected to Redis {}", name);
    Ok(conn_manager)
}
