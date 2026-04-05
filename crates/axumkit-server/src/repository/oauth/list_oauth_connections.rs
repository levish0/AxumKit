use axumkit_entity::user_oauth_connections::{
    Column as OAuthConnectionsColumn, Entity as OAuthConnectionsEntity,
    Model as OAuthConnectionModel,
};
use axumkit_errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, Order, QueryFilter, QueryOrder};
use uuid::Uuid;

/// Queries all OAuth connections for a user.
pub async fn repository_list_oauth_connections_by_user_id<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<Vec<OAuthConnectionModel>, Errors>
where
    C: ConnectionTrait,
{
    let connections = OAuthConnectionsEntity::find()
        .filter(OAuthConnectionsColumn::UserId.eq(user_id))
        .order_by(OAuthConnectionsColumn::Id, Order::Asc) // UUIDv7 is time-sortable
        .all(conn)
        .await?;

    Ok(connections)
}
