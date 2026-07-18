use crate::actor::ActorResponse;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use constants::NotificationAction;
use entity::common::NotificationType;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
/// Response payload for notification response.
pub struct NotificationResponse {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<ActorResponse>,
    pub notification_type: NotificationType,
    pub action: NotificationAction,
    pub board_id: Option<Uuid>,
    pub post_id: Option<Uuid>,
    pub comment_id: Option<Uuid>,
    pub additional_data: Option<serde_json::Value>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
/// Response payload for notification list response.
pub struct NotificationListResponse {
    pub data: Vec<NotificationResponse>,
    /// Whether there are newer (more recent) notifications
    pub has_newer: bool,
    /// Whether there are older notifications
    pub has_older: bool,
}

#[derive(Debug, Serialize, ToSchema)]
/// Response payload for unread count response.
pub struct UnreadCountResponse {
    pub count: u64,
}

impl IntoResponse for NotificationListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl IntoResponse for UnreadCountResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
