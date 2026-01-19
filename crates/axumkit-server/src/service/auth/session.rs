use crate::service::auth::session_types::Session;
use axumkit_config::ServerConfig;
use axumkit_errors::errors::Errors;
use chrono::Utc;
use redis::AsyncCommands;
use redis::aio::ConnectionManager as RedisClient;

pub struct SessionService;

impl SessionService {
    /// 세션 Redis key
    fn session_key(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    /// 사용자별 세션 목록 Redis key
    fn user_sessions_key(user_id: &str) -> String {
        format!("user_sessions:{}", user_id)
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

        // Redis TTL = sliding TTL (활동 기반 만료)
        let ttl_seconds = (config.auth_session_sliding_ttl_hours * 3600) as u64;
        // Set TTL = max_lifetime + 1시간 여유분 (세션보다 오래 유지)
        let set_ttl_seconds = config.auth_session_max_lifetime_hours * 3600 + 3600;

        let json = serde_json::to_string(&session).map_err(|e| {
            Errors::SysInternalError(format!("Session serialization failed: {}", e))
        })?;

        // 파이프라인으로 세션 저장 + user_sessions Set 추가 + Set TTL 설정 (1 RTT)
        let mut conn = redis.clone();
        let user_sessions_key = Self::user_sessions_key(&user_id);

        redis::pipe()
            .set_ex(Self::session_key(&session.session_id), json, ttl_seconds)
            .ignore()
            .sadd(&user_sessions_key, &session.session_id)
            .ignore()
            .expire(&user_sessions_key, set_ttl_seconds)
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

    pub async fn delete_session(redis: &RedisClient, session_id: &str) -> Result<(), Errors> {
        let mut conn = redis.clone();
        let key = Self::session_key(session_id);

        // 먼저 세션에서 user_id를 가져옴 (보안: 외부에서 user_id를 받지 않음)
        let session_data: Option<String> = conn.get(&key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session retrieval failed: {}", e))
        })?;

        match session_data {
            Some(data) => {
                let session: Session = serde_json::from_str(&data).map_err(|e| {
                    Errors::SysInternalError(format!("Session deserialization failed: {}", e))
                })?;

                // 파이프라인으로 세션 키 삭제 + user_sessions Set에서 제거 (1 RTT)
                redis::pipe()
                    .del(&key)
                    .ignore()
                    .srem(Self::user_sessions_key(&session.user_id), session_id)
                    .ignore()
                    .query_async::<()>(&mut conn)
                    .await
                    .map_err(|e| {
                        Errors::SysInternalError(format!("Redis session deletion failed: {}", e))
                    })?;
            }
            None => {
                // 세션이 이미 만료/삭제됨 - 무시
            }
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

        // Redis TTL 계산
        let ttl_seconds = (new_expires_at - now).num_seconds().max(0) as u64;
        // Set TTL = max_lifetime + 1시간 여유분 (세션 활동 시 연장)
        let set_ttl_seconds = config.auth_session_max_lifetime_hours * 3600 + 3600;

        let mut refreshed_session = session.clone();
        refreshed_session.expires_at = new_expires_at;

        let json = serde_json::to_string(&refreshed_session).map_err(|e| {
            Errors::SysInternalError(format!("Session serialization failed: {}", e))
        })?;

        // 파이프라인으로 세션 갱신 + Set TTL 연장 (1 RTT)
        let mut conn = redis.clone();
        let user_sessions_key = Self::user_sessions_key(&session.user_id);

        redis::pipe()
            .set_ex(Self::session_key(&session.session_id), json, ttl_seconds)
            .ignore()
            .expire(&user_sessions_key, set_ttl_seconds)
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

        // 연장이 필요하고 연장 가능한 경우에만 실행
        if session.needs_refresh(config.auth_session_refresh_threshold) && session.can_refresh() {
            Self::refresh_session(redis, session).await
        } else {
            Ok(None)
        }
    }

    /// 특정 사용자의 모든 세션 삭제 (비밀번호 재설정 시 사용)
    pub async fn delete_all_user_sessions(
        redis: &RedisClient,
        user_id: &str,
    ) -> Result<u64, Errors> {
        let mut conn = redis.clone();
        let user_sessions_key = Self::user_sessions_key(user_id);

        // Set에서 모든 session_id 조회
        let session_ids: Vec<String> = conn
            .smembers(&user_sessions_key)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Failed to get user sessions: {}", e)))?;

        let count = session_ids.len() as u64;

        if count > 0 {
            // 파이프라인으로 모든 세션 키 + user_sessions Set 한 번에 삭제 (1 RTT)
            let mut pipe = redis::pipe();
            for session_id in &session_ids {
                pipe.del(Self::session_key(session_id)).ignore();
            }
            pipe.del(&user_sessions_key).ignore();

            pipe.query_async::<()>(&mut conn).await.map_err(|e| {
                Errors::SysInternalError(format!("Failed to delete user sessions: {}", e))
            })?;
        }

        Ok(count)
    }

    /// 현재 세션을 제외한 모든 세션 삭제 (비밀번호 변경 시 사용)
    pub async fn delete_other_sessions(
        redis: &RedisClient,
        user_id: &str,
        current_session_id: &str,
    ) -> Result<u64, Errors> {
        let mut conn = redis.clone();
        let user_sessions_key = Self::user_sessions_key(user_id);

        // Set에서 모든 session_id 조회
        let session_ids: Vec<String> = conn
            .smembers(&user_sessions_key)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Failed to get user sessions: {}", e)))?;

        // 현재 세션 제외
        let other_session_ids: Vec<&String> = session_ids
            .iter()
            .filter(|id| id.as_str() != current_session_id)
            .collect();

        let count = other_session_ids.len() as u64;

        if count > 0 {
            // 파이프라인으로 다른 세션들만 삭제 (1 RTT)
            let mut pipe = redis::pipe();
            for session_id in &other_session_ids {
                pipe.del(Self::session_key(session_id)).ignore();
                pipe.srem(&user_sessions_key, session_id.as_str()).ignore();
            }

            pipe.query_async::<()>(&mut conn).await.map_err(|e| {
                Errors::SysInternalError(format!("Failed to delete other sessions: {}", e))
            })?;
        }

        Ok(count)
    }
}
