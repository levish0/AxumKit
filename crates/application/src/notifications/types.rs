use super::entities::{notification, push_device};
use crate::notifications::PushPlatform;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct NotificationSummary {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub organization_id: Uuid,
    pub post_id: Uuid,
    pub comment_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}
impl From<notification::Model> for NotificationSummary {
    fn from(row: notification::Model) -> Self {
        Self {
            id: row.id,
            actor_id: row.actor_id,
            organization_id: row.organization_id,
            post_id: row.post_id,
            comment_id: row.comment_id,
            created_at: row.created_at,
            read_at: row.read_at,
        }
    }
}

#[derive(Clone, Copy)]
pub struct NotificationPreferences {
    pub post_comments: bool,
    pub push_enabled: bool,
}
impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            post_comments: true,
            push_enabled: false,
        }
    }
}

pub struct PushDeviceSummary {
    pub id: Uuid,
    pub platform: PushPlatform,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
impl From<push_device::Model> for PushDeviceSummary {
    fn from(row: push_device::Model) -> Self {
        Self {
            id: row.id,
            platform: row.platform,
            created_at: row.created_at,
            expires_at: row.expires_at,
        }
    }
}
