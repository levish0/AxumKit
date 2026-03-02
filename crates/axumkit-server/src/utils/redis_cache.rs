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

/// Store JSON payload with TTL using a token-derived key.
pub async fn store_json_for_token_with_ttl<T, F>(
    redis_client: &RedisClient,
    token: &str,
    key_from_token: F,
    value: &T,
    ttl_seconds: u64,
) -> Result<(), Errors>
where
    T: Serialize,
    F: Fn(&str) -> String,
{
    let key = key_from_token(token);
    set_json_with_ttl(redis_client, &key, value, ttl_seconds).await
}

/// Generate a token, derive key from it, and store JSON payload with TTL.
pub async fn issue_token_and_store_json_with_ttl<T, FToken, FKey>(
    redis_client: &RedisClient,
    token_factory: FToken,
    key_from_token: FKey,
    value: &T,
    ttl_seconds: u64,
) -> Result<String, Errors>
where
    T: Serialize,
    FToken: FnOnce() -> String,
    FKey: Fn(&str) -> String,
{
    let token = token_factory();
    store_json_for_token_with_ttl(redis_client, &token, key_from_token, value, ttl_seconds).await?;
    Ok(token)
}

/// Retrieve a JSON value from Redis (uncompressed)
pub async fn get_json<T: DeserializeOwned>(
    redis_client: &RedisClient,
    key: &str,
) -> Result<Option<T>, Errors> {
    let mut conn = redis_client.clone();
    let data: Option<String> = conn.get(key).await.map_err(|e| {
        Errors::SysInternalError(format!("Redis read failed for key '{}': {}", key, e))
    })?;

    match data {
        Some(json_str) => {
            let value = serde_json::from_str(&json_str).map_err(|e| {
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

/// Retrieve a JSON value from Redis and delete the key atomically.
/// Returns caller-provided domain errors for missing/invalid payloads.
pub async fn get_json_and_delete<T, FMissing, FInvalid>(
    redis_client: &RedisClient,
    key: &str,
    missing_error: FMissing,
    invalid_json_error: FInvalid,
) -> Result<T, Errors>
where
    T: DeserializeOwned,
    FMissing: Fn() -> Errors,
    FInvalid: Fn(serde_json::Error) -> Errors,
{
    let mut conn = redis_client.clone();
    let data: Option<String> = conn.get_del(key).await.map_err(|e| {
        Errors::SysInternalError(format!(
            "Redis read-and-delete failed for key '{}': {}",
            key, e
        ))
    })?;

    let json = match data {
        Some(json) => json,
        None => return Err(missing_error()),
    };

    serde_json::from_str(&json).map_err(invalid_json_error)
}

/// Retrieve an optional JSON value from Redis and delete the key atomically.
/// Returns `Ok(None)` when the key is missing.
pub async fn get_optional_json_and_delete<T, FInvalid>(
    redis_client: &RedisClient,
    key: &str,
    invalid_json_error: FInvalid,
) -> Result<Option<T>, Errors>
where
    T: DeserializeOwned,
    FInvalid: Fn(serde_json::Error) -> Errors,
{
    let mut conn = redis_client.clone();
    let data: Option<String> = conn.get_del(key).await.map_err(|e| {
        Errors::SysInternalError(format!(
            "Redis read-and-delete failed for key '{}': {}",
            key, e
        ))
    })?;

    match data {
        Some(json) => {
            let value = serde_json::from_str(&json).map_err(invalid_json_error)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Delete a key from Redis
pub async fn delete_key(redis_client: &RedisClient, key: &str) -> Result<(), Errors> {
    let mut conn = redis_client.clone();
    conn.del::<_, ()>(key).await.map_err(|e| {
        Errors::SysInternalError(format!("Redis delete failed for key '{}': {}", key, e))
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
