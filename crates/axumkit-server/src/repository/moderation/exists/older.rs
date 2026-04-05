use super::super::filter::{ModerationLogFilter, apply_moderation_log_filter};
use axumkit_entity::moderation_logs::{
    Column as ModerationLogColumn, Entity as ModerationLogEntity,
};
use axumkit_errors::errors::Errors;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
};
use uuid::Uuid;

pub async fn repository_exists_older_moderation_log<C>(
    conn: &C,
    filter: &ModerationLogFilter,
    cursor_id: Uuid,
) -> Result<bool, Errors>
where
    C: ConnectionTrait,
{
    let query = apply_moderation_log_filter(
        ModerationLogEntity::find().filter(ModerationLogColumn::Id.lt(cursor_id)),
        filter,
    );

    let count = query.limit(1).count(conn).await?;
    Ok(count > 0)
}
