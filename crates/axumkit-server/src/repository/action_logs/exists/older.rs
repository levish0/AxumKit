use super::super::filter::{ActionLogFilter, apply_action_log_filter};
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
    let query = apply_action_log_filter(
        ActionLogEntity::find().filter(ActionLogColumn::Id.lt(cursor_id)),
        filter,
    );

    let count = query.limit(1).count(conn).await?;
    Ok(count > 0)
}
