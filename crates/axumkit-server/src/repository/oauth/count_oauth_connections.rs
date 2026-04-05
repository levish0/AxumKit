use axumkit_entity::user_oauth_connections::{
    Column as OAuthConnectionsColumn, Entity as OAuthConnectionsEntity,
};
use axumkit_errors::errors::Errors;
use sea_orm::PaginatorTrait;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Queries the number of OAuth connections for a user.
/// Used to protect the last authentication method.
pub async fn repository_count_oauth_connections<C>(conn: &C, user_id: Uuid) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let count = OAuthConnectionsEntity::find()
        .filter(OAuthConnectionsColumn::UserId.eq(user_id))
        .count(conn)
        .await?;

    Ok(count)
}
