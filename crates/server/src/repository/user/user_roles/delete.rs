use chrono::Utc;
use entity::common::Role;
use entity::user_roles::{Column, Entity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

pub async fn repository_delete_user_role<C>(
    conn: &C,
    user_id: Uuid,
    role: Role,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = Entity::delete_many()
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Role.eq(role))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}

pub async fn repository_delete_expired_user_role<C>(
    conn: &C,
    user_id: Uuid,
    role: Role,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let now = Utc::now();

    let result = Entity::delete_many()
        .filter(Column::UserId.eq(user_id))
        .filter(Column::Role.eq(role))
        .filter(Column::ExpiresAt.is_not_null())
        .filter(Column::ExpiresAt.lte(now))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}
