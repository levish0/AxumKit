use chrono::Utc;
use entity::user_bans::{Column, Entity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

pub async fn repository_delete_user_ban<C>(conn: &C, user_id: Uuid) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = Entity::delete_many()
        .filter(Column::UserId.eq(user_id))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}

/// Deletes only expired bans for the user (prevents UNIQUE constraint violation on re-ban).
pub async fn repository_delete_expired_user_ban<C>(conn: &C, user_id: Uuid) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let now = Utc::now();

    let result = Entity::delete_many()
        .filter(Column::UserId.eq(user_id))
        .filter(Column::ExpiresAt.is_not_null())
        .filter(Column::ExpiresAt.lte(now))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}
