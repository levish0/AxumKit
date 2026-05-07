use entity::common::OAuthProvider;
use entity::user_oauth_connections::{
    Column as OAuthConnectionsColumn, Entity as OAuthConnectionsEntity,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Deletes a specific provider's OAuth connection for a user.
pub async fn repository_delete_oauth_connection<C>(
    conn: &C,
    user_id: Uuid,
    provider: OAuthProvider,
) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    let result = OAuthConnectionsEntity::delete_many()
        .filter(OAuthConnectionsColumn::UserId.eq(user_id))
        .filter(OAuthConnectionsColumn::Provider.eq(provider))
        .exec(conn)
        .await?;

    if result.rows_affected == 0 {
        return Err(Errors::OauthConnectionNotFound);
    }

    Ok(())
}
