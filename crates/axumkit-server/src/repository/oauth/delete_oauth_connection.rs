use axumkit_entity::common::OAuthProvider;
use axumkit_entity::user_oauth_connections::{
    Column as OAuthConnectionsColumn, Entity as OAuthConnectionsEntity,
};
use axumkit_errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// 특정 사용자의 특정 provider OAuth 연결을 삭제합니다.
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
