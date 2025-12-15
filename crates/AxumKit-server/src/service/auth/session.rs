use crate::config::db_config::DbConfig;
use crate::dto::auth::internal::session::Session;
use crate::errors::errors::Errors;
use crate::utils::redis_cache::set_json_with_ttl;
use redis::AsyncCommands;
use redis::aio::ConnectionManager as RedisClient;

pub struct SessionService;

impl SessionService {
    pub async fn create_session(
        redis: &RedisClient,
        user_id: String,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Result<Session, Errors> {
        let config = DbConfig::get();
        let session = Session::new(user_id, config.auth_session_expire_time)
            .with_client_info(user_agent, ip_address);

        let ttl_seconds = (config.auth_session_expire_time * 3600) as u64; // hours to seconds
        set_json_with_ttl(redis, &session.redis_key(), &session, ttl_seconds).await?;

        Ok(session)
    }

    pub async fn get_session(
        redis: &RedisClient,
        session_id: &str,
    ) -> Result<Option<Session>, Errors> {
        let mut conn = redis.clone();
        let key = format!("session:{}", session_id);

        let session_data: Option<String> = conn.get(&key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session retrieval failed: {}", e))
        })?;

        match session_data {
            Some(data) => {
                let session: Session = serde_json::from_str(&data).map_err(|e| {
                    Errors::SysInternalError(format!("Session deserialization failed: {}", e))
                })?;

                if session.is_expired() {
                    Self::delete_session(redis, session_id).await?;
                    Ok(None)
                } else {
                    Ok(Some(session))
                }
            }
            None => Ok(None),
        }
    }

    pub async fn delete_session(redis: &RedisClient, session_id: &str) -> Result<(), Errors> {
        let mut conn = redis.clone();
        let key = format!("session:{}", session_id);

        conn.del::<_, ()>(&key).await.map_err(|e| {
            Errors::SysInternalError(format!("Redis session deletion failed: {}", e))
        })?;

        Ok(())
    }

    pub async fn refresh_session(
        redis: &RedisClient,
        session_id: &str,
    ) -> Result<Option<Session>, Errors> {
        if let Some(mut session) = Self::get_session(redis, session_id).await? {
            let config = DbConfig::get();
            let now = chrono::Utc::now();
            session.expires_at = now + chrono::Duration::hours(config.auth_session_expire_time);

            let ttl_seconds = (config.auth_session_expire_time * 3600) as u64; // hours to seconds
            set_json_with_ttl(redis, &session.redis_key(), &session, ttl_seconds).await?;

            Ok(Some(session))
        } else {
            Ok(None)
        }
    }
}
