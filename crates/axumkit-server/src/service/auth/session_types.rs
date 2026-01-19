use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub user_id: Uuid,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub max_expires_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

impl Session {
    pub fn new(user_id: String, sliding_ttl_hours: i64, max_lifetime_hours: i64) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::hours(sliding_ttl_hours);
        let max_expires_at = now + Duration::hours(max_lifetime_hours);

        Self {
            session_id: Uuid::now_v7().to_string(),
            user_id,
            created_at: now,
            expires_at,
            max_expires_at,
            user_agent: None,
            ip_address: None,
        }
    }

    pub fn with_client_info(
        mut self,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Self {
        self.user_agent = user_agent;
        self.ip_address = ip_address;
        self
    }

    /// 세션을 연장할 수 있는지 확인 (최대 수명 체크)
    pub fn can_refresh(&self) -> bool {
        Utc::now() < self.max_expires_at
    }

    /// 세션 연장이 필요한지 확인 (TTL 임계값 체크)
    pub fn needs_refresh(&self, threshold_percent: u8) -> bool {
        let now = Utc::now();
        let remaining = (self.expires_at - now).num_seconds();
        let total_ttl = (self.expires_at - self.created_at).num_seconds();

        if remaining <= 0 || total_ttl <= 0 {
            return false;
        }

        let remaining_percent = (remaining as f64 / total_ttl as f64 * 100.0) as u8;
        remaining_percent <= threshold_percent
    }
}
