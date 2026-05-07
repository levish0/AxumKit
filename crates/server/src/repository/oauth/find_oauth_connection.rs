use entity::common::OAuthProvider;
use entity::user_oauth_connections::{
    Column as OAuthConnectionsColumn, Entity as OAuthConnectionsEntity,
    Model as OAuthConnectionModel,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Queries a specific provider's OAuth connection for a user.
pub async fn repository_find_oauth_connection<C>(
    conn: &C,
    user_id: Uuid,
    provider: OAuthProvider,
) -> Result<Option<OAuthConnectionModel>, Errors>
where
    C: ConnectionTrait,
{
    let connection = OAuthConnectionsEntity::find()
        .filter(OAuthConnectionsColumn::UserId.eq(user_id))
        .filter(OAuthConnectionsColumn::Provider.eq(provider))
        .one(conn)
        .await?;

    Ok(connection)
}
