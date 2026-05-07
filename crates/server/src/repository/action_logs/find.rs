use super::filter::{ActionLogFilter, apply_action_log_filter};
use dto::pagination::CursorDirection;
use entity::action_logs::{
    Column as ActionLogColumn, Entity as ActionLogEntity, Model as ActionLogModel,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

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
    let mut query = apply_action_log_filter(ActionLogEntity::find(), filter);

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
