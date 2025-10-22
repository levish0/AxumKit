use chrono::{DateTime, Utc};
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
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

impl Session {
    pub fn new(user_id: String, expire_hours: i64) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(expire_hours);

        Self {
            session_id: Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            expires_at,
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

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn redis_key(&self) -> String {
        format!("session:{}", self.session_id)
    }
}