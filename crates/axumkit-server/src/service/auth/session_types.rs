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

    pub fn can_refresh(&self) -> bool {
        Utc::now() < self.max_expires_at
    }

    pub fn needs_refresh(&self, threshold_percent: u8, sliding_ttl_hours: i64) -> bool {
        let now = Utc::now();
        let remaining = (self.expires_at - now).num_seconds();

        if remaining <= 0 {
            return false;
        }

        let sliding_ttl_seconds = Duration::hours(sliding_ttl_hours).num_seconds();
        if sliding_ttl_seconds <= 0 {
            return false;
        }

        let threshold_percent = threshold_percent.min(100) as i64;
        let threshold_seconds = (sliding_ttl_seconds * threshold_percent) / 100;

        remaining <= threshold_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_session(expires_in_hours: i64) -> Session {
        let now = Utc::now();
        Session {
            session_id: "test-session".to_string(),
            user_id: "test-user".to_string(),
            created_at: now,
            expires_at: now + Duration::hours(expires_in_hours),
            max_expires_at: now + Duration::hours(720),
            user_agent: None,
            ip_address: None,
        }
    }

    #[test]
    fn needs_refresh_uses_sliding_ttl_threshold() {
        let session = make_session(168);
        assert!(!session.needs_refresh(50, 168));

        let near_expiry = make_session(80);
        assert!(near_expiry.needs_refresh(50, 168));
    }

    #[test]
    fn needs_refresh_is_not_affected_by_session_age() {
        let mut session = make_session(160);
        session.created_at = Utc::now() - Duration::days(20);

        assert!(!session.needs_refresh(50, 168));
    }

    #[test]
    fn needs_refresh_returns_false_for_invalid_sliding_ttl() {
        let session = make_session(10);
        assert!(!session.needs_refresh(50, 0));
    }
}
