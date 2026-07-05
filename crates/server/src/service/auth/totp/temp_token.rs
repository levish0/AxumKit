use crate::utils::crypto::token::hash_token;
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
    /// Device-cookie token carried from the initial login, so new-device verification can run
    /// after the TOTP step (browser flow). `None` for the native-app flow.
    #[serde(default)]
    pub device_token: Option<String>,
    /// Whether to apply new-device verification after TOTP: `true` for browser, `false` for app.
    #[serde(default)]
    pub apply_device_check: bool,
    pub created_at: DateTime<Utc>,
}

impl TotpTempToken {
    pub fn new(
        user_id: Uuid,
        user_agent: Option<String>,
        ip_address: Option<String>,
        remember_me: bool,
        device_token: Option<String>,
        apply_device_check: bool,
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
            device_token,
            apply_device_check,
            created_at: Utc::now(),
        }
    }

    /// Redis key for the temp token.
    ///
    /// The raw token never lives at rest: it is hashed (blake3) into the key, so a Redis snapshot
    /// yields only non-replayable hashes. The raw token is returned to the client and echoed back
    /// on verify, matching the hash-at-rest discipline used by sessions / reset / email-change.
    pub fn redis_key(&self) -> String {
        format!("totp_temp:{}", hash_token(&self.token))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        redis: &RedisClient,
        user_id: Uuid,
        user_agent: Option<String>,
        ip_address: Option<String>,
        remember_me: bool,
        device_token: Option<String>,
        apply_device_check: bool,
    ) -> Result<Self, Errors> {
        let temp_token = Self::new(
            user_id,
            user_agent,
            ip_address,
            remember_me,
            device_token,
            apply_device_check,
        );

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
        // Look up by the hashed token id (raw token never lives at rest).
        let key = format!("totp_temp:{}", hash_token(token));

        get_optional_json_and_delete(redis, &key, |e| {
            Errors::SysInternalError(format!("TOTP temp token deserialization failed: {}", e))
        })
        .await
    }
}
