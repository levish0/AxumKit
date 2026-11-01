use application::notifications::{NotificationPreferences, NotificationSummary, PushDeviceSummary};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    PostComment,
}

#[derive(Serialize, ToSchema)]
pub struct NotificationOutput {
    pub id: Uuid,
    pub kind: NotificationKind,
    pub actor_id: Uuid,
    pub organization_id: Uuid,
    pub post_id: Uuid,
    pub comment_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}
impl From<NotificationSummary> for NotificationOutput {
    fn from(row: NotificationSummary) -> Self {
        Self {
            id: row.id,
            kind: NotificationKind::PostComment,
            actor_id: row.actor_id,
            organization_id: row.organization_id,
            post_id: row.post_id,
            comment_id: row.comment_id,
            created_at: row.created_at,
            read_at: row.read_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct NotificationPreferencesOutput {
    pub post_comments: bool,
    pub push_enabled: bool,
}
impl From<NotificationPreferences> for NotificationPreferencesOutput {
    fn from(value: NotificationPreferences) -> Self {
        Self {
            post_comments: value.post_comments,
            push_enabled: value.push_enabled,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PushDeviceOutput {
    pub id: Uuid,
    pub platform: PushPlatform,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
impl From<PushDeviceSummary> for PushDeviceOutput {
    fn from(row: PushDeviceSummary) -> Self {
        Self {
            id: row.id,
            platform: row.platform.into(),
            created_at: row.created_at,
            expires_at: row.expires_at,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PushPlatform {
    Android,
    Ios,
    Web,
}
impl From<application::notifications::PushPlatform> for PushPlatform {
    fn from(value: application::notifications::PushPlatform) -> Self {
        match value {
            application::notifications::PushPlatform::Android => Self::Android,
            application::notifications::PushPlatform::Ios => Self::Ios,
            application::notifications::PushPlatform::Web => Self::Web,
        }
    }
}
impl From<PushPlatform> for application::notifications::PushPlatform {
    fn from(value: PushPlatform) -> Self {
        match value {
            PushPlatform::Android => Self::Android,
            PushPlatform::Ios => Self::Ios,
            PushPlatform::Web => Self::Web,
        }
    }
}
