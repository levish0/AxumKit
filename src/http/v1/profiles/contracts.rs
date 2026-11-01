use application::profiles::ProfileSummary;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ProfileOutput {
    pub account_id: Uuid,
    pub display_name: String,
    pub bio: String,
    pub version: i64,
    pub created_at: DateTime<Utc>,
}
impl From<ProfileSummary> for ProfileOutput {
    fn from(row: ProfileSummary) -> Self {
        Self {
            account_id: row.account_id,
            display_name: row.display_name,
            bio: row.bio,
            version: row.version,
            created_at: row.created_at,
        }
    }
}
