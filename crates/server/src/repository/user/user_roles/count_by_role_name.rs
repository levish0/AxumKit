use chrono::Utc;
use entity::common::Role;
use entity::user_roles::{Column, Entity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter};

/// Counts active users assigned to a given role
pub async fn repository_count_active_user_roles_by_role_name<C>(
    conn: &C,
    role: Role,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let now = Utc::now();

    let count = Entity::find()
        .filter(Column::Role.eq(role))
        .filter(Column::ExpiresAt.is_null().or(Column::ExpiresAt.gt(now)))
        .count(conn)
        .await?;

    Ok(count)
}
