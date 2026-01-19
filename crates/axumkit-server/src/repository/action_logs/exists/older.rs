use super::super::find::ActionLogFilter;
use axumkit_entity::action_logs::{Column as ActionLogColumn, Entity as ActionLogEntity};
use axumkit_errors::errors::Errors;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
};
use uuid::Uuid;

pub async fn repository_exists_older_action_log<C>(
    conn: &C,
    filter: &ActionLogFilter,
    cursor_id: Uuid,
) -> Result<bool, Errors>
where
    C: ConnectionTrait,
{
    let mut query = ActionLogEntity::find().filter(ActionLogColumn::Id.lt(cursor_id));

    if let Some(actor_id) = filter.actor_id {
        query = query.filter(ActionLogColumn::ActorId.eq(actor_id));
    }

    if let Some(actor_ip) = filter.actor_ip {
        query = query.filter(ActionLogColumn::ActorIp.eq(actor_ip));
    }

    if let Some(resource_id) = filter.resource_id {
        query = query.filter(ActionLogColumn::ResourceId.eq(resource_id));
    }

    if let Some(ref resource_type) = filter.resource_type {
        query = query.filter(ActionLogColumn::ResourceType.eq(resource_type.clone()));
    }

    if let Some(actions) = &filter.actions {
        if !actions.is_empty() {
            let action_strs: Vec<&str> = actions.iter().map(|a| a.as_str()).collect();
            query = query.filter(ActionLogColumn::Action.is_in(action_strs));
        }
    }

    let count = query.limit(1).count(conn).await?;
    Ok(count > 0)
}
