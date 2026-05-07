use crate::utils::redis_cache::{get_optional_json_and_delete, set_json_with_ttl};
use chrono::{DateTime, Utc};
use errors::errors::Errors;
use rand::Rng;
use redis::aio::ConnectionManager as RedisClient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TEMP_TOKEN_TTL_SECONDS: u64 = 120; // 2 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpTempToken {
    pub token: String,
    pub user_id: Uuid,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub remember_me: bool,
    pub created_at: DateTime<Utc>,
}

impl TotpTempToken {
    pub fn new(
        user_id: Uuid,
        user_agent: Option<String>,
        ip_address: Option<String>,
        remember_me: bool,
    ) -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let token = hex::encode(bytes);

        Self {
            token,
            user_id,
            user_agent,
            ip_address,
            remember_me,
            created_at: Utc::now(),
        }
    }

    pub fn redis_key(&self) -> String {
        format!("totp_temp:{}", self.token)
    }

    pub async fn create(
        redis: &RedisClient,
        user_id: Uuid,
        user_agent: Option<String>,
        ip_address: Option<String>,
        remember_me: bool,
    ) -> Result<Self, Errors> {
        let temp_token = Self::new(user_id, user_agent, ip_address, remember_me);

        set_json_with_ttl(
            redis,
            &temp_token.redis_key(),
            &temp_token,
            TEMP_TOKEN_TTL_SECONDS,
        )
        .await?;

        Ok(temp_token)
    }

    pub async fn get_and_delete(redis: &RedisClient, token: &str) -> Result<Option<Self>, Errors> {
        let key = format!("totp_temp:{}", token);

        get_optional_json_and_delete(redis, &key, |e| {
            Errors::SysInternalError(format!("TOTP temp token deserialization failed: {}", e))
        })
        .await
    }
}
