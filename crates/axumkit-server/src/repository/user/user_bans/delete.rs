use axumkit_entity::user_bans::{Column, Entity};
use axumkit_errors::errors::Errors;
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
