use axumkit_constants::ActionLogAction;
use axumkit_entity::action_logs::{ActiveModel as ActionLogActiveModel, Model as ActionLogModel};
use axumkit_entity::common::ActionResourceType;
use axumkit_errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub async fn repository_create_action_log<C>(
    conn: &C,
    action: ActionLogAction,
    actor_id: Option<Uuid>,
    actor_ip: Option<IpNetwork>,
    resource_type: ActionResourceType,
    resource_id: Option<Uuid>,
    summary: String,
    metadata: Option<JsonValue>,
) -> Result<ActionLogModel, Errors>
where
    C: ConnectionTrait,
{
    let log = ActionLogActiveModel {
        id: Default::default(),
        action: Set(action.as_str().to_string()),
        actor_id: Set(actor_id),
        actor_ip: Set(actor_ip),
        resource_type: Set(resource_type),
        resource_id: Set(resource_id),
        summary: Set(summary),
        metadata: Set(metadata),
        created_at: Default::default(), // Uses database default now()
    };

    let log = log.insert(conn).await?;

    Ok(log)
}
