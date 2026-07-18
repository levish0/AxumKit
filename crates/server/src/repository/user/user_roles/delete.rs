use chrono::Utc;
use entity::common::Role;
use entity::user_roles::{Column, Entity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Deletes a specific role for a user (including expired entries)
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

/// Deletes all roles for a user (privilege revocation on account deletion).
///
/// Keeps a deactivated account from retaining admin/mod privileges or appearing in role listings.
pub async fn repository_delete_all_user_roles<C>(conn: &C, user_id: Uuid) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = Entity::delete_many()
        .filter(Column::UserId.eq(user_id))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}

/// Deletes only expired entries of a specific role for a user (avoids UNIQUE conflicts on grant)
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
