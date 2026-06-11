use crate::service::auth::session_types::Session;
use chrono::Utc;
use config::ServerConfig;
use errors::errors::Errors;
use redis::AsyncCommands;
use redis::aio::ConnectionManager as RedisClient;

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

    /// 새 세션을 생성해 Redis에 저장한다.
    ///
    /// # 역할
    /// - 세션 ID/만료 정보를 포함한 `Session`을 생성한다.
    /// - 세션 payload, 관리 ID lookup, 사용자별 ZSET 인덱스를 저장한다.
    ///
    /// # 연계
    /// - `Session::new`
    /// - Redis pipeline (`session:*`, `session_mgmt:*`, `user_sessions:*`)
    ///
    /// # Errors
    /// - 직렬화/Redis 저장 실패 시 `Errors::SysInternalError`
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

        Ok(session)
    }

    /// 세션 ID로 세션 payload를 조회한다.
    ///
    /// # 역할
    /// Redis에서 세션 JSON을 읽어 `Session`으로 역직렬화한다.
    /// 세션이 없으면 `Ok(None)`을 반환한다.
    ///
    /// # Errors
    /// - 역직렬화/Redis 조회 실패 시 `Errors::SysInternalError`
    pub async fn get_session(
        redis: &RedisClient,
        session_id: &str,
    ) -> Result<Option<Session>, Errors> {
        let mut conn = redis.clone();
        let key = Self::session_key(session_id);

        let session_data: Option<String> = conn.get(&key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session retrieval failed: {}", e))
        })?;

        // Redis TTL이 만료를 처리하므로 키가 존재하면 유효한 세션
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

    /// 세션 한 건을 삭제한다.
    ///
    /// # 역할
    /// 세션 payload에서 사용자 ID와 관리 ID를 읽은 뒤 관련 Redis 키를 함께 제거한다.
    /// 이미 만료/삭제된 세션이면 no-op으로 처리한다.
    ///
    /// # Errors
    /// - 역직렬화/Redis 삭제 실패 시 `Errors::SysInternalError`
    pub async fn delete_session(redis: &RedisClient, session_id: &str) -> Result<(), Errors> {
        let mut conn = redis.clone();
        let key = Self::session_key(session_id);

        // Read user_id from stored session payload; never trust external user_id.
        let session_data: Option<String> = conn.get(&key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session retrieval failed: {}", e))
        })?;

        if let Some(data) = session_data {
            let session: Session = serde_json::from_str(&data).map_err(|e| {
                Errors::SysInternalError(format!("Session deserialization failed: {}", e))
            })?;

            let management_key = Self::session_management_key(&session.management_id);
            // Delete session payload + management lookup + per-user index member.
            let user_sessions_key = Self::user_sessions_key(&session.user_id);

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

        Ok(())
    }

    /// 세션 TTL 연장 (최대 수명 체크 포함)
    pub async fn refresh_session(
        redis: &RedisClient,
        session: &Session,
    ) -> Result<Option<Session>, Errors> {
        let config = ServerConfig::get();
        let now = Utc::now();

        // 최대 수명 초과 시 연장 불가
        if now >= session.max_expires_at {
            return Ok(None);
        }

        // 새 만료 시간 = min(now + sliding_ttl, max_expires_at)
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

    /// 조건부 세션 연장 (임계값 체크 + 최대 수명 체크)
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

    /// 특정 사용자의 활성 세션 목록을 조회한다.
    ///
    /// # 역할
    /// - 사용자 인덱스에서 세션 ID를 수집한다.
    /// - 각 `session:{id}` payload를 읽어 만료/누락된 항목은 제외한다.
    /// - `created_at` 내림차순으로 정렬해 반환한다.
    ///
    /// # Errors
    /// - 역직렬화/Redis 조회 실패 시 `Errors::SysInternalError`
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

        // MGET으로 한 번에 payload 조회
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

        // 최신 세션이 위로 오도록 정렬
        sessions.sort_by_key(|session| std::cmp::Reverse(session.created_at));

        Ok(sessions)
    }

    /// 소유권을 검증한 뒤 세션 한 건을 삭제한다.
    ///
    /// # 역할
    /// 세션 payload의 `user_id`가 요청자와 일치할 때만 삭제한다.
    /// 다른 사용자의 세션을 revoke하려는 시도는 `Errors::NotFound`로 응답해 존재 자체를 노출하지 않는다.
    ///
    /// # Errors
    /// - 세션이 없거나 다른 사용자 소유면 `Errors::NotFound`
    /// - Redis 실패 시 `Errors::SysInternalError`
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

        // 다른 사용자 세션은 존재를 알리지 않도록 동일 에러로 매핑
        if session.user_id != user_id || session.management_id != management_id {
            return Err(Errors::NotFound("Session not found".to_string()));
        }

        Self::delete_session(redis, &session_id).await
    }

    /// 특정 사용자의 모든 세션 삭제 (비밀번호 재설정 시 사용)
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

    /// 현재 세션을 제외한 모든 세션 삭제 (비밀번호 변경 시 사용)
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
