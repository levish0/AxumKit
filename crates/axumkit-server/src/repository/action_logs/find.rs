use axumkit_constants::ActionLogAction;
use axumkit_dto::pagination::CursorDirection;
use axumkit_entity::action_logs::{
    Column as ActionLogColumn, Entity as ActionLogEntity, Model as ActionLogModel,
};
use axumkit_entity::common::ActionResourceType;
use axumkit_errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

#[derive(Debug, Default, Clone)]
pub struct ActionLogFilter {
    pub actor_id: Option<Uuid>,
    pub actor_ip: Option<IpNetwork>,
    pub resource_id: Option<Uuid>,
    pub resource_type: Option<ActionResourceType>,
    pub actions: Option<Vec<ActionLogAction>>,
}

pub async fn repository_find_action_logs<C>(
    conn: &C,
    filter: &ActionLogFilter,
    cursor_id: Option<Uuid>,
    cursor_direction: Option<CursorDirection>,
    limit: u64,
) -> Result<Vec<ActionLogModel>, Errors>
where
    C: ConnectionTrait,
{
    let mut query = ActionLogEntity::find();

    if let Some(actor_id) = filter.actor_id {
        query = query.filter(ActionLogColumn::ActorId.eq(actor_id));
    }

    if let Some(actor_ip) = filter.actor_ip {
        query = query.filter(ActionLogColumn::ActorIp.eq(actor_ip));
    }

    if let Some(resource_id) = filter.resource_id {
        query = query.filter(ActionLogColumn::ResourceId.eq(resource_id));
    }

    if let Some(resource_type) = &filter.resource_type {
        query = query.filter(ActionLogColumn::ResourceType.eq(resource_type.clone()));
    }

    if let Some(actions) = &filter.actions {
        if !actions.is_empty() {
            let action_strs: Vec<&str> = actions.iter().map(|a| a.as_str()).collect();
            query = query.filter(ActionLogColumn::Action.is_in(action_strs));
        }
    }

    // Apply cursor-based filtering (UUIDv7 is time-sortable)
    if let Some(id) = cursor_id {
        let direction = cursor_direction.unwrap_or(CursorDirection::Older);
        query = match direction {
            CursorDirection::Older => query
                .filter(ActionLogColumn::Id.lt(id))
                .order_by_desc(ActionLogColumn::Id),
            CursorDirection::Newer => query
                .filter(ActionLogColumn::Id.gt(id))
                .order_by_asc(ActionLogColumn::Id),
        };
    } else {
        query = query.order_by_desc(ActionLogColumn::Id);
    }

    let logs = query.limit(limit).all(conn).await?;

    Ok(logs)
}
