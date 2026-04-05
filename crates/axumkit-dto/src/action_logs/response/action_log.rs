use chrono::{DateTime, Utc};
use axumkit_entity::action_logs::Model as ActionLogModel;
use axumkit_entity::common::ActionResourceType;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
/// Response payload for action log response.
pub struct ActionLogResponse {
    pub id: Uuid,
    pub action: String,
    pub actor_id: Option<Uuid>,
    pub resource_type: ActionResourceType,
    pub resource_id: Option<Uuid>,
    pub summary: String,
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

impl From<ActionLogModel> for ActionLogResponse {
    fn from(model: ActionLogModel) -> Self {
        Self {
            id: model.id,
            action: model.action,
            actor_id: model.actor_id,
            resource_type: model.resource_type,
            resource_id: model.resource_id,
            summary: model.summary,
            metadata: model.metadata,
            created_at: model.created_at,
        }
    }
}