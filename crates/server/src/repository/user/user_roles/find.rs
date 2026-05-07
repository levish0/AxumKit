use entity::common::Role;
use entity::user_roles::{Column, Entity};
use errors::errors::Errors;
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter};
use uuid::Uuid;

pub async fn repository_find_user_roles<C>(conn: &C, user_id: Uuid) -> Result<Vec<Role>, Errors>
where
    C: ConnectionTrait,
{
    let now = Utc::now();

    let mut roles = Entity::find()
        .filter(Column::UserId.eq(user_id))
        .filter(Column::ExpiresAt.is_null().or(Column::ExpiresAt.gt(now)))
        .all(conn)
        .await?
        .into_iter()
        .map(|entry| entry.role)
        .collect::<Vec<_>>();

    roles.sort_by_key(|role| std::cmp::Reverse(role.display_priority()));

    Ok(roles)
}
