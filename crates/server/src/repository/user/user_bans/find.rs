use entity::user_bans::{Column, Entity, Model};
use errors::errors::Errors;
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter};
use uuid::Uuid;

pub async fn repository_find_user_ban<C>(conn: &C, user_id: Uuid) -> Result<Option<Model>, Errors>
where
    C: ConnectionTrait,
{
    let now = Utc::now();

    let ban = Entity::find()
        .filter(Column::UserId.eq(user_id))
        .filter(Column::ExpiresAt.is_null().or(Column::ExpiresAt.gt(now)))
        .one(conn)
        .await?;

    Ok(ban)
}

pub async fn repository_is_user_banned<C>(conn: &C, user_id: Uuid) -> Result<bool, Errors>
where
    C: ConnectionTrait,
{
    let ban = repository_find_user_ban(conn, user_id).await?;
    Ok(ban.is_some())
}
