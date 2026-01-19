use axumkit_entity::common::OAuthProvider;
use axumkit_entity::user_oauth_connections::{
    Column as OAuthConnectionsColumn, Entity as OAuthConnectionsEntity,
    Model as OAuthConnectionModel,
};
use axumkit_errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// 특정 사용자의 특정 provider OAuth 연결을 조회합니다.
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
