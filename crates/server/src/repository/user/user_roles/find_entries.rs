use chrono::Utc;
use entity::user_roles::{Column, Entity, Model};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter};
use uuid::Uuid;

/// Fetches a user's active role entries (non-expired, includes expires_at)
pub async fn repository_find_user_role_entries<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<Vec<Model>, Errors>
where
    C: ConnectionTrait,
{
    let now = Utc::now();

    let entries = Entity::find()
        .filter(Column::UserId.eq(user_id))
        .filter(Column::ExpiresAt.is_null().or(Column::ExpiresAt.gt(now)))
        .all(conn)
        .await?;

    Ok(entries)
}
