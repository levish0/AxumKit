use constants::ModerationAction;
use entity::common::ModerationResourceType;
use entity::moderation_logs::{
    Column as ModerationLogColumn, Entity as ModerationLogEntity,
};
use sea_orm::{ColumnTrait, QueryFilter, Select};
use uuid::Uuid;

#[derive(Debug, Default, Clone)]
pub struct ModerationLogFilter {
    pub actor_id: Option<Uuid>,
    pub resource_type: Option<ModerationResourceType>,
    pub resource_id: Option<Uuid>,
    pub actions: Option<Vec<ModerationAction>>,
}

pub(crate) fn apply_moderation_log_filter(
    mut query: Select<ModerationLogEntity>,
    filter: &ModerationLogFilter,
) -> Select<ModerationLogEntity> {
    if let Some(actor_id) = filter.actor_id {
        query = query.filter(ModerationLogColumn::ActorId.eq(actor_id));
    }

    if let Some(resource_type) = filter.resource_type.clone() {
        query = query.filter(ModerationLogColumn::ResourceType.eq(resource_type));
    }

    if let Some(resource_id) = filter.resource_id {
        query = query.filter(ModerationLogColumn::ResourceId.eq(resource_id));
    }

    if let Some(actions) = &filter.actions
        && !actions.is_empty()
    {
        let action_strs: Vec<&str> = actions.iter().map(|action| action.as_str()).collect();
        query = query.filter(ModerationLogColumn::Action.is_in(action_strs));
    }

    query
}
