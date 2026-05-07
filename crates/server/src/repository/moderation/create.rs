use constants::ModerationAction;
use entity::common::ModerationResourceType;
use entity::moderation_logs::{
    ActiveModel as ModerationLogActiveModel, Model as ModerationLogModel,
};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub async fn repository_create_moderation_log<C>(
    conn: &C,
    action: ModerationAction,
    actor_id: Option<Uuid>,
    resource_type: ModerationResourceType,
    resource_id: Option<Uuid>,
    reason: String,
    metadata: Option<JsonValue>,
) -> Result<ModerationLogModel, Errors>
where
    C: ConnectionTrait,
{
    let log = ModerationLogActiveModel {
        id: Default::default(),
        action: Set(action.as_str().to_string()),
        actor_id: Set(actor_id),
        resource_type: Set(resource_type),
        resource_id: Set(resource_id),
        reason: Set(reason),
        metadata: Set(metadata),
        created_at: Default::default(),
    };

    let log = log.insert(conn).await?;

    Ok(log)
}
