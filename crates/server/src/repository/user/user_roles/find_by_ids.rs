use chrono::Utc;
use entity::user_roles::{Column, Entity, Model};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter};
use uuid::Uuid;

/// Batch-fetches active role entries for multiple users (non-expired only)
///
/// Used to enrich many users' roles in a single query, e.g. for search results.
/// Group the returned entries by `user_id` when consuming them.
pub async fn repository_find_user_role_entries_by_ids<C>(
    conn: &C,
    user_ids: &[Uuid],
) -> Result<Vec<Model>, Errors>
where
    C: ConnectionTrait,
{
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }

    let now = Utc::now();

    let entries = Entity::find()
        .filter(Column::UserId.is_in(user_ids.iter().copied()))
        .filter(Column::ExpiresAt.is_null().or(Column::ExpiresAt.gt(now)))
        .all(conn)
        .await?;

    Ok(entries)
}
