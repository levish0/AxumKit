use axum::{Json, response::IntoResponse};
use axumkit_entity::common::ModerationResourceType;
use axumkit_entity::moderation_logs::Model as ModerationLogModel;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value as JsonValue;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ModerationLogListItem {
    pub id: Uuid,
    pub action: String,
    pub actor_id: Option<Uuid>,
    pub resource_type: ModerationResourceType,
    pub resource_id: Option<Uuid>,
    pub reason: String,
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

impl From<ModerationLogModel> for ModerationLogListItem {
    fn from(model: ModerationLogModel) -> Self {
        Self {
            id: model.id,
            action: model.action,
            actor_id: model.actor_id,
            resource_type: model.resource_type,
            resource_id: model.resource_id,
            reason: model.reason,
            metadata: model.metadata,
            created_at: model.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListModerationLogsResponse {
    pub data: Vec<ModerationLogListItem>,
    pub has_newer: bool,
    pub has_older: bool,
}

impl IntoResponse for ListModerationLogsResponse {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
