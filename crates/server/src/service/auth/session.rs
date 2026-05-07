use crate::service::auth::session_types::Session;
use config::ServerConfig;
use errors::errors::Errors;
use chrono::Utc;
use redis::AsyncCommands;
use redis::aio::ConnectionManager as RedisClient;
use std::collections::HashSet;

pub struct SessionService;

impl SessionService {
    /// Session payload key.
    fn session_key(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    /// Per-session TTL-synced index key.
    fn user_session_index_key(user_id: &str, session_id: &str) -> String {
        format!("user_session_idx:{}:{}", user_id, session_id)
    }

    fn user_session_index_prefix(user_id: &str) -> String {
        format!("user_session_idx:{}:", user_id)
    }

    /// Collect active session IDs from TTL-synced index keys.
    async fn collect_user_session_ids(
        redis: &RedisClient,
        user_id: &str,
    ) -> Result<Vec<String>, Errors> {
        let mut conn = redis.clone();
        let key_prefix = Self::user_session_index_prefix(user_id);
        let scan_pattern = format!("{}*", key_prefix);
        let mut cursor = 0_u64;
        let mut session_ids = HashSet::new();

        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&scan_pattern)
                .arg("COUNT")
                .arg(200_u32)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    Errors::SysInternalError(format!(
                        "Failed to scan user session index '{}': {}",
                        scan_pattern, e
                    ))
                })?;

            for key in keys {
                if let Some(session_id) = key.strip_prefix(&key_prefix) {
                    if !session_id.is_empty() {
                        session_ids.insert(session_id.to_string());
                    }
                }
            }

            if next_cursor == 0 {
                break;
            }
            cursor = next_cursor;
        }

        Ok(session_ids.into_iter().collect())
    }

    pub async fn create_session(
        redis: &RedisClient,
        user_id: String,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Result<Session, Errors> {
        let config = ServerConfig::get();
        let session = Session::new(
            user_id.clone(),
            config.auth_session_sliding_ttl_hours,
            config.auth_session_max_lifetime_hours,
        )
        .with_client_info(user_agent, ip_address);

        // Redis TTL = sliding TTL.
        let ttl_seconds = (config.auth_session_sliding_ttl_hours * 3600) as u64;

        let json = serde_json::to_string(&session).map_err(|e| {
            Errors::SysInternalError(format!("Session serialization failed: {}", e))
        })?;

        // Store session + TTL-synced per-session index key.
        let mut conn = redis.clone();
        let session_key = Self::session_key(&session.session_id);
        let index_key = Self::user_session_index_key(&user_id, &session.session_id);

        redis::pipe()
            .set_ex(&session_key, json, ttl_seconds)
            .ignore()
            .set_ex(&index_key, "1", ttl_seconds)
            .ignore()
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Failed to create session: {}", e)))?;

        Ok(session)
    }

    pub async fn get_session(
        redis: &RedisClient,
        session_id: &str,
    ) -> Result<Option<Session>, Errors> {
        let mut conn = redis.clone();
        let key = Self::session_key(session_id);

        let session_data: Option<String> = conn.get(&key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session retrieval failed: {}", e))
        })?;

        match session_data {
            Some(data) => {
                let session: Session = serde_json::from_str(&data).map_err(|e| {
                    Errors::SysInternalError(format!("Session deserialization failed: {}", e))
                })?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    pub async fn delete_session(redis: &RedisClient, session_id: &str) -> Result<(), Errors> {
        let mut conn = redis.clone();
        let key = Self::session_key(session_id);

        // Read user_id from stored session payload; never trust external user_id.
        let session_data: Option<String> = conn.get(&key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session retrieval failed: {}", e))
        })?;

        match session_data {
            Some(data) => {
                let session: Session = serde_json::from_str(&data).map_err(|e| {
                    Errors::SysInternalError(format!("Session deserialization failed: {}", e))
                })?;

                let index_key = Self::user_session_index_key(&session.user_id, session_id);
                // Delete session payload + index key.
                redis::pipe()
                    .del(&key)
                    .ignore()
                    .del(&index_key)
                    .ignore()
                    .query_async::<()>(&mut conn)
                    .await
                    .map_err(|e| {
                        Errors::SysInternalError(format!("Redis session deletion failed: {}", e))
                    })?;
            }
            None => {
                // Session already expired/deleted.
            }
        }

        Ok(())
    }

    pub async fn refresh_session(
        redis: &RedisClient,
        session: &Session,
    ) -> Result<Option<Session>, Errors> {
        let config = ServerConfig::get();
        let now = Utc::now();

        if now >= session.max_expires_at {
            return Ok(None);
        }

        let sliding_expiry = now + chrono::Duration::hours(config.auth_session_sliding_ttl_hours);
        let new_expires_at = sliding_expiry.min(session.max_expires_at);

        // Redis TTL
        let ttl_seconds = (new_expires_at - now).num_seconds().max(0) as u64;
        if ttl_seconds == 0 {
            return Ok(None);
        }

        let mut refreshed_session = session.clone();
        refreshed_session.expires_at = new_expires_at;

        let json = serde_json::to_string(&refreshed_session).map_err(|e| {
            Errors::SysInternalError(format!("Session serialization failed: {}", e))
        })?;

        // Refresh session payload and per-session index TTL in one pipeline.
        let mut conn = redis.clone();
        let session_key = Self::session_key(&session.session_id);
        let index_key = Self::user_session_index_key(&session.user_id, &session.session_id);

        redis::pipe()
            .set_ex(&session_key, json, ttl_seconds)
            .ignore()
            .set_ex(&index_key, "1", ttl_seconds)
            .ignore()
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Failed to refresh session: {}", e)))?;

        Ok(Some(refreshed_session))
    }

    pub async fn maybe_refresh_session(
        redis: &RedisClient,
        session: &Session,
    ) -> Result<Option<Session>, Errors> {
        let config = ServerConfig::get();

        // Refresh only when threshold is hit and max lifetime still allows it.
        if session.needs_refresh(
            config.auth_session_refresh_threshold,
            config.auth_session_sliding_ttl_hours,
        ) && session.can_refresh()
        {
            Self::refresh_session(redis, session).await
        } else {
            Ok(None)
        }
    }

    pub async fn delete_all_user_sessions(
        redis: &RedisClient,
        user_id: &str,
    ) -> Result<u64, Errors> {
        let mut conn = redis.clone();

        let session_ids = Self::collect_user_session_ids(redis, user_id).await?;

        let count = session_ids.len() as u64;
        let mut pipe = redis::pipe();

        for session_id in &session_ids {
            pipe.del(Self::session_key(session_id)).ignore();
            pipe.del(Self::user_session_index_key(user_id, session_id))
                .ignore();
        }

        pipe.query_async::<()>(&mut conn).await.map_err(|e| {
            Errors::SysInternalError(format!("Failed to delete user sessions: {}", e))
        })?;

        Ok(count)
    }

    pub async fn delete_other_sessions(
        redis: &RedisClient,
        user_id: &str,
        current_session_id: &str,
    ) -> Result<u64, Errors> {
        let mut conn = redis.clone();

        let session_ids = Self::collect_user_session_ids(redis, user_id).await?;

        // Keep current session, delete all others.
        let other_session_ids: Vec<&String> = session_ids
            .iter()
            .filter(|id| id.as_str() != current_session_id)
            .collect();

        let count = other_session_ids.len() as u64;

        if count > 0 {
            // Delete other sessions and prune index keys.
            let mut pipe = redis::pipe();
            for session_id in &other_session_ids {
                pipe.del(Self::session_key(session_id)).ignore();
                pipe.del(Self::user_session_index_key(user_id, session_id.as_str()))
                    .ignore();
            }

            pipe.query_async::<()>(&mut conn).await.map_err(|e| {
                Errors::SysInternalError(format!("Failed to delete other sessions: {}", e))
            })?;
        }

        Ok(count)
    }
}
