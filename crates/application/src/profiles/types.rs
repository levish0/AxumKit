use super::entities::profile;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct SaveProfile {
    pub display_name: String,
    pub bio: String,
    pub expected_version: i64,
}

pub struct ProfileSummary {
    pub account_id: Uuid,
    pub display_name: String,
    pub bio: String,
    pub version: i64,
    pub created_at: DateTime<Utc>,
}
impl From<profile::Model> for ProfileSummary {
    fn from(value: profile::Model) -> Self {
        Self {
            account_id: value.account_id,
            display_name: value.display_name,
            bio: value.bio,
            version: value.version,
            created_at: value.created_at,
        }
    }
}
