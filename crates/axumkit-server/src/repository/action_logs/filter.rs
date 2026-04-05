use axumkit_constants::ActionLogAction;
use axumkit_entity::action_logs::{Column as ActionLogColumn, Entity as ActionLogEntity};
use axumkit_entity::common::ActionResourceType;
use sea_orm::{ColumnTrait, QueryFilter, Select};
use uuid::Uuid;

#[derive(Debug, Default, Clone)]
pub struct ActionLogFilter {
    pub actor_id: Option<Uuid>,
    pub resource_id: Option<Uuid>,
    pub resource_type: Option<ActionResourceType>,
    pub actions: Option<Vec<ActionLogAction>>,
}

pub(crate) fn apply_action_log_filter(
    mut query: Select<ActionLogEntity>,
    filter: &ActionLogFilter,
) -> Select<ActionLogEntity> {
    if let Some(actor_id) = filter.actor_id {
        query = query.filter(ActionLogColumn::ActorId.eq(actor_id));
    }

    if let Some(resource_id) = filter.resource_id {
        query = query.filter(ActionLogColumn::ResourceId.eq(resource_id));
    }

    if let Some(resource_type) = filter.resource_type {
        query = query.filter(ActionLogColumn::ResourceType.eq(resource_type));
    }

    if let Some(actions) = &filter.actions
        && !actions.is_empty()
    {
        let action_strs: Vec<&str> = actions.iter().map(|a| a.as_str()).collect();
        query = query.filter(ActionLogColumn::Action.is_in(action_strs));
    }

    query
}
