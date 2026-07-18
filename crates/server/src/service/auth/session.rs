use crate::repository::user::repository_find_user_by_id;
use crate::service::auth::session_types::{Session, SessionContext};
use crate::utils::crypto::token::{generate_secure_token, hash_token};
use chrono::Utc;
use config::ServerConfig;
use errors::errors::Errors;
use redis::AsyncCommands;
use redis::aio::ConnectionManager as RedisClient;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

/// Data structure for session service.
pub struct SessionService;

impl SessionService {
    /// Session payload key.
    fn session_key(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    /// Management ID -> bearer session ID lookup key.
    fn session_management_key(management_id: &str) -> String {
        format!("session_mgmt:{}", management_id)
    }

    /// Per-user active session index.
    fn user_sessions_key(user_id: &str) -> String {
        format!("user_sessions:{}", user_id)
    }

    fn user_sessions_key_ttl_seconds(config: &ServerConfig) -> u64 {
        let max_lifetime = (config.auth_session_max_lifetime_hours * 3600).max(1) as u64;
        let sliding_ttl = (config.auth_session_sliding_ttl_hours * 3600).max(1) as u64;
        max_lifetime.max(sliding_ttl)
    }

    /// Collect active session management IDs from the per-user ZSET.
    async fn collect_user_management_ids(
        redis: &RedisClient,
        user_id: &str,
    ) -> Result<Vec<String>, Errors> {
        let mut conn = redis.clone();
        let key = Self::user_sessions_key(user_id);
        let now = Utc::now().timestamp();

        redis::cmd("ZREMRANGEBYSCORE")
            .arg(&key)
            .arg("-inf")
            .arg(now)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| {
                Errors::SysInternalError(format!(
                    "Failed to prune expired user session index '{}': {}",
                    key, e
                ))
            })?;

        redis::cmd("ZRANGE")
            .arg(&key)
            .arg(0)
            .arg(-1)
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                Errors::SysInternalError(format!(
                    "Failed to read user session index '{}': {}",
                    key, e
                ))
            })
    }

    async fn remove_user_management_ids(
        redis: &RedisClient,
        user_id: &str,
        management_ids: &[String],
    ) -> Result<(), Errors> {
        if management_ids.is_empty() {
            return Ok(());
        }

        let mut conn = redis.clone();
        let mut pipe = redis::pipe();
        let user_sessions_key = Self::user_sessions_key(user_id);

        pipe.cmd("ZREM")
            .arg(&user_sessions_key)
            .arg(management_ids)
            .ignore();

        for management_id in management_ids {
            pipe.del(Self::session_management_key(management_id))
                .ignore();
        }

        pipe.query_async::<()>(&mut conn).await.map_err(|e| {
            Errors::SysInternalError(format!("Failed to prune stale user sessions: {}", e))
        })?;

        Ok(())
    }

    /// Creates a new session and stores it in Redis.
    ///
    /// # Role
    /// - Builds a `Session` containing the session ID and expiration info.
    /// - Stores the session payload, management ID lookup, and per-user ZSET index.
    ///
    /// # Related
    /// - `Session::new`
    /// - Redis pipeline (`session:*`, `session_mgmt:*`, `user_sessions:*`)
    ///
    /// # Returns
    /// `(raw_token, session)`: the **raw** 256-bit bearer token to put in the
    /// cookie (returned to the caller and never stored), and the stored [`Session`]
    /// whose `session_id` is the *hash* of that token. Everything in Redis is keyed
    /// by the hash, so a store leak yields no usable session tokens.
    ///
    /// # Errors
    /// - `Errors::SysInternalError` on serialization or Redis write failure
    pub async fn create_session(
        redis: &RedisClient,
        user_id: String,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Result<(String, Session), Errors> {
        let config = ServerConfig::get();
        // Raw bearer token (cookie only) and its hash (the server-side identifier).
        let raw_token = generate_secure_token();
        let session_id = hash_token(&raw_token);
        let session = Session::new(
            session_id,
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

        // Store session + management lookup + per-user session index.
        let mut conn = redis.clone();
        let session_key = Self::session_key(&session.session_id);
        let management_key = Self::session_management_key(&session.management_id);
        let user_sessions_key = Self::user_sessions_key(&user_id);
        let user_sessions_key_ttl = Self::user_sessions_key_ttl_seconds(config);

        redis::pipe()
            .set_ex(&session_key, json, ttl_seconds)
            .ignore()
            .set_ex(&management_key, &session.session_id, ttl_seconds)
            .ignore()
            .cmd("ZADD")
            .arg(&user_sessions_key)
            .arg(session.expires_at.timestamp())
            .arg(&session.management_id)
            .ignore()
            .cmd("EXPIRE")
            .arg(&user_sessions_key)
            .arg(user_sessions_key_ttl)
            .ignore()
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Failed to create session: {}", e)))?;

        Ok((raw_token, session))
    }

    /// Looks up a session payload by session ID.
    ///
    /// # Role
    /// Reads the session JSON from Redis and deserializes it into a `Session`.
    /// Returns `Ok(None)` if the session does not exist.
    ///
    /// # Errors
    /// - `Errors::SysInternalError` on deserialization or Redis read failure
    pub async fn get_session(
        redis: &RedisClient,
        session_id: &str,
    ) -> Result<Option<Session>, Errors> {
        let mut conn = redis.clone();
        let key = Self::session_key(session_id);

        let session_data: Option<String> = conn.get(&key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session retrieval failed: {}", e))
        })?;

        // Redis TTL handles expiration, so if the key exists the session is valid
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

    /// Validates a session ID and resolves it into a `SessionContext`.
    ///
    /// # Role
    /// The **single authority** that checks both the session store and the DB. All
    /// session extractors resolve sessions through this function.
    /// - Looks up the session in the store and checks absolute expiration.
    /// - Checks whether the user is active (soft-delete); if not, cleans up leftover sessions.
    /// - Attempts a sliding refresh once the threshold is reached (failures are only logged).
    ///
    /// # Returns
    /// - `Ok(Some(_))` for a valid, active session
    /// - `Ok(None)` when no usable session exists (missing from store / inactive user)
    ///
    /// # Errors
    /// - `Errors::SessionExpired` for expired sessions
    /// - `Errors::SessionInvalidUserId` if the stored user_id fails to parse
    /// - Store/DB errors
    pub async fn resolve_session(
        redis: &RedisClient,
        db: &DatabaseConnection,
        session_token: &str,
    ) -> Result<Option<SessionContext>, Errors> {
        // The cookie carries the raw bearer token; everything server-side is keyed
        // by its hash, so resolve to the stored id before any lookup.
        let session_id = hash_token(session_token);
        let Some(session) = Self::get_session(redis, &session_id).await? else {
            return Ok(None);
        };

        // Absolute expiration (independent of the sliding Redis TTL).
        if Utc::now() >= session.max_expires_at {
            return Err(Errors::SessionExpired);
        }

        let user_id =
            Uuid::parse_str(&session.user_id).map_err(|_| Errors::SessionInvalidUserId)?;

        // Reject if the user is gone or soft-deleted: a deactivated account must not stay
        // authenticated even if the best-effort session purge on deletion failed.
        let is_active = repository_find_user_by_id(db, user_id)
            .await?
            .is_some_and(|user| user.deleted_at.is_none());
        if !is_active {
            if let Err(e) = Self::delete_session(redis, &session_id).await {
                tracing::warn!(error = ?e, "Failed to delete stale session for deleted user");
            }
            return Ok(None);
        }

        // Conditionally refresh the session (sliding expiration). Errors are logged, not fatal.
        if let Err(e) = Self::maybe_refresh_session(redis, &session).await {
            tracing::warn!(error = ?e, "Failed to refresh session");
        }

        Ok(Some(SessionContext {
            user_id,
            session_id,
            management_id: session.management_id,
        }))
    }

    /// Deletes a single session.
    ///
    /// # Role
    /// Reads the user ID and management ID from the session payload, then removes the
    /// related Redis keys together. A session that is already expired/deleted is a no-op.
    ///
    /// # Errors
    /// - `Errors::SysInternalError` on deserialization or Redis delete failure
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

                let management_key = Self::session_management_key(&session.management_id);
                let user_sessions_key = Self::user_sessions_key(&session.user_id);
                // Delete session payload + management lookup + per-user index member.
                redis::pipe()
                    .del(&key)
                    .ignore()
                    .del(&management_key)
                    .ignore()
                    .cmd("ZREM")
                    .arg(&user_sessions_key)
                    .arg(&session.management_id)
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

    /// Extends the session TTL (includes max lifetime check)
    pub async fn refresh_session(
        redis: &RedisClient,
        session: &Session,
    ) -> Result<Option<Session>, Errors> {
        let config = ServerConfig::get();
        let now = Utc::now();

        // Cannot extend past the maximum lifetime
        if now >= session.max_expires_at {
            return Ok(None);
        }

        // New expiration = min(now + sliding_ttl, max_expires_at)
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

        // Refresh session payload, management lookup, and per-user index in one pipeline.
        let mut conn = redis.clone();
        let session_key = Self::session_key(&session.session_id);
        let management_key = Self::session_management_key(&session.management_id);
        let user_sessions_key = Self::user_sessions_key(&session.user_id);
        let user_sessions_key_ttl = Self::user_sessions_key_ttl_seconds(config);

        redis::pipe()
            .set_ex(&session_key, json, ttl_seconds)
            .ignore()
            .set_ex(&management_key, &session.session_id, ttl_seconds)
            .ignore()
            .cmd("ZADD")
            .arg(&user_sessions_key)
            .arg(refreshed_session.expires_at.timestamp())
            .arg(&session.management_id)
            .ignore()
            .cmd("EXPIRE")
            .arg(&user_sessions_key)
            .arg(user_sessions_key_ttl)
            .ignore()
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Failed to refresh session: {}", e)))?;

        Ok(Some(refreshed_session))
    }

    /// Conditionally extends the session (threshold check + max lifetime check)
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

    /// Lists a user's active sessions.
    ///
    /// # Role
    /// - Collects session IDs from the per-user index.
    /// - Reads each `session:{id}` payload, skipping expired/missing entries.
    /// - Returns the sessions sorted by `created_at` descending.
    ///
    /// # Errors
    /// - `Errors::SysInternalError` on deserialization or Redis read failure
    pub async fn list_user_sessions(
        redis: &RedisClient,
        user_id: &str,
    ) -> Result<Vec<Session>, Errors> {
        let mut conn = redis.clone();
        let management_ids = Self::collect_user_management_ids(redis, user_id).await?;

        if management_ids.is_empty() {
            return Ok(Vec::new());
        }

        let management_keys: Vec<String> = management_ids
            .iter()
            .map(|id| Self::session_management_key(id))
            .collect();
        let raw_session_ids: Vec<Option<String>> =
            conn.mget(&management_keys).await.map_err(|e| {
                Errors::SysInternalError(format!("Failed to fetch session management keys: {}", e))
            })?;

        let mut stale_management_ids = Vec::new();
        let mut session_refs = Vec::new();
        for (management_id, session_id) in management_ids.into_iter().zip(raw_session_ids) {
            match session_id {
                Some(session_id) => session_refs.push((management_id, session_id)),
                None => stale_management_ids.push(management_id),
            }
        }

        if session_refs.is_empty() {
            Self::remove_user_management_ids(redis, user_id, &stale_management_ids).await?;
            return Ok(Vec::new());
        }

        // Fetch all payloads at once with MGET
        let keys: Vec<String> = session_refs
            .iter()
            .map(|(_, session_id)| Self::session_key(session_id))
            .collect();
        let raw_payloads: Vec<Option<String>> = conn.mget(&keys).await.map_err(|e| {
            Errors::SysInternalError(format!("Failed to fetch user sessions: {}", e))
        })?;

        let mut sessions = Vec::new();
        for ((management_id, _session_id), payload) in session_refs.into_iter().zip(raw_payloads) {
            let Some(data) = payload else {
                stale_management_ids.push(management_id);
                continue;
            };

            match serde_json::from_str::<Session>(&data) {
                Ok(session)
                    if session.user_id == user_id && session.management_id == management_id =>
                {
                    sessions.push(session);
                }
                _ => stale_management_ids.push(management_id),
            }
        }

        Self::remove_user_management_ids(redis, user_id, &stale_management_ids).await?;

        // Sort so the most recent sessions come first
        sessions.sort_by_key(|session| std::cmp::Reverse(session.created_at));

        Ok(sessions)
    }

    /// Verifies ownership, then deletes a single session.
    ///
    /// # Role
    /// Deletes only when the session payload's `user_id` matches the requester.
    /// Attempts to revoke another user's session respond with `Errors::NotFound`
    /// so as not to reveal the session's existence.
    ///
    /// # Errors
    /// - `Errors::NotFound` if the session is missing or owned by another user
    /// - `Errors::SysInternalError` on Redis failure
    pub async fn revoke_user_session(
        redis: &RedisClient,
        user_id: &str,
        management_id: &str,
    ) -> Result<(), Errors> {
        let mut conn = redis.clone();
        let management_key = Self::session_management_key(management_id);
        let session_id: Option<String> = conn.get(&management_key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session retrieval failed: {}", e))
        })?;
        let session_id =
            session_id.ok_or_else(|| Errors::NotFound("Session not found".to_string()))?;

        let session = Self::get_session(redis, &session_id)
            .await?
            .ok_or_else(|| Errors::NotFound("Session not found".to_string()))?;

        // Map another user's session to the same error to avoid revealing its existence
        if session.user_id != user_id || session.management_id != management_id {
            return Err(Errors::NotFound("Session not found".to_string()));
        }

        Self::delete_session(redis, &session_id).await
    }

    /// Deletes all sessions for a user (used on password reset)
    pub async fn delete_all_user_sessions(
        redis: &RedisClient,
        user_id: &str,
    ) -> Result<u64, Errors> {
        let mut conn = redis.clone();

        let management_ids = Self::collect_user_management_ids(redis, user_id).await?;

        if management_ids.is_empty() {
            return Ok(0);
        }

        let management_keys: Vec<String> = management_ids
            .iter()
            .map(|id| Self::session_management_key(id))
            .collect();
        let raw_session_ids: Vec<Option<String>> =
            conn.mget(&management_keys).await.map_err(|e| {
                Errors::SysInternalError(format!("Failed to fetch user session IDs: {}", e))
            })?;
        let session_ids: Vec<String> = raw_session_ids.into_iter().flatten().collect();

        let count = session_ids.len() as u64;
        let mut pipe = redis::pipe();

        for session_id in &session_ids {
            pipe.del(Self::session_key(session_id)).ignore();
        }
        for management_id in &management_ids {
            pipe.del(Self::session_management_key(management_id))
                .ignore();
        }
        pipe.del(Self::user_sessions_key(user_id)).ignore();

        pipe.query_async::<()>(&mut conn).await.map_err(|e| {
            Errors::SysInternalError(format!("Failed to delete user sessions: {}", e))
        })?;

        Ok(count)
    }

    /// Deletes all sessions except the current one (used on password change)
    pub async fn delete_other_sessions(
        redis: &RedisClient,
        user_id: &str,
        current_session_id: &str,
    ) -> Result<u64, Errors> {
        let mut conn = redis.clone();

        let management_ids = Self::collect_user_management_ids(redis, user_id).await?;

        if management_ids.is_empty() {
            return Ok(0);
        }

        let management_keys: Vec<String> = management_ids
            .iter()
            .map(|id| Self::session_management_key(id))
            .collect();
        let raw_session_ids: Vec<Option<String>> =
            conn.mget(&management_keys).await.map_err(|e| {
                Errors::SysInternalError(format!("Failed to fetch user session IDs: {}", e))
            })?;

        let mut stale_management_ids = Vec::new();
        let mut other_sessions = Vec::new();
        for (management_id, session_id) in management_ids.into_iter().zip(raw_session_ids) {
            match session_id {
                Some(session_id) if session_id != current_session_id => {
                    other_sessions.push((management_id, session_id));
                }
                Some(_) => {}
                None => stale_management_ids.push(management_id),
            }
        }

        let count = other_sessions.len() as u64;

        if count > 0 {
            // Delete other sessions and prune index members.
            let mut pipe = redis::pipe();
            for (management_id, session_id) in &other_sessions {
                pipe.del(Self::session_key(session_id)).ignore();
                pipe.del(Self::session_management_key(management_id))
                    .ignore();
                pipe.cmd("ZREM")
                    .arg(Self::user_sessions_key(user_id))
                    .arg(management_id)
                    .ignore();
            }

            pipe.query_async::<()>(&mut conn).await.map_err(|e| {
                Errors::SysInternalError(format!("Failed to delete other sessions: {}", e))
            })?;
        }

        Self::remove_user_management_ids(redis, user_id, &stale_management_ids).await?;

        Ok(count)
    }
}
