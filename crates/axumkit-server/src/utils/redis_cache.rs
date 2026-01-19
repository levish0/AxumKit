use axumkit_errors::errors::Errors;
use redis::AsyncCommands;
use redis::aio::ConnectionManager as RedisClient;
use serde::{Serialize, de::DeserializeOwned};

/// Cache a JSON-serialized value in Redis with TTL
pub async fn set_json_with_ttl<T: Serialize>(
    redis_client: &RedisClient,
    key: &str,
    value: &T,
    ttl_seconds: u64,
) -> Result<(), Errors> {
    let json = serde_json::to_string(value).map_err(|e| {
        Errors::SysInternalError(format!(
            "JSON serialization failed for Redis key '{}': {}",
            key, e
        ))
    })?;

    let mut conn = redis_client.clone();
    conn.set_ex::<_, _, ()>(key, json, ttl_seconds)
        .await
        .map_err(|e| {
            Errors::SysInternalError(format!("Redis write failed for key '{}': {}", key, e))
        })?;

    Ok(())
}

/// Compress data using zstd
fn compress_data(data: &[u8]) -> Result<Vec<u8>, Errors> {
    zstd::encode_all(data, 3)
        .map_err(|e| Errors::SysInternalError(format!("Compression failed: {}", e)))
}

/// Decompress data using zstd
fn decompress_data(compressed: &[u8]) -> Result<Vec<u8>, Errors> {
    zstd::decode_all(compressed)
        .map_err(|e| Errors::SysInternalError(format!("Decompression failed: {}", e)))
}

/// Cache a JSON-serialized value in Redis with TTL and compression
pub async fn set_json_compressed<T: Serialize>(
    redis_client: &RedisClient,
    key: &str,
    value: &T,
    ttl_seconds: u64,
) -> Result<(), Errors> {
    let json_bytes = serde_json::to_vec(value).map_err(|e| {
        Errors::SysInternalError(format!(
            "JSON serialization failed for Redis key '{}': {}",
            key, e
        ))
    })?;

    let compressed = compress_data(&json_bytes)?;

    let mut conn = redis_client.clone();
    conn.set_ex::<_, _, ()>(key, compressed, ttl_seconds)
        .await
        .map_err(|e| {
            Errors::SysInternalError(format!("Redis write failed for key '{}': {}", key, e))
        })?;

    Ok(())
}

/// Retrieve a compressed JSON value from Redis
pub async fn get_json_compressed<T: DeserializeOwned>(
    redis_client: &RedisClient,
    key: &str,
) -> Result<Option<T>, Errors> {
    let mut conn = redis_client.clone();
    let data: Option<Vec<u8>> = conn.get(key).await.map_err(|e| {
        Errors::SysInternalError(format!("Redis read failed for key '{}': {}", key, e))
    })?;

    match data {
        Some(compressed) => {
            let json_bytes = decompress_data(&compressed)?;
            let value = serde_json::from_slice(&json_bytes).map_err(|e| {
                Errors::SysInternalError(format!(
                    "JSON deserialization failed for Redis key '{}': {}",
                    key, e
                ))
            })?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}
