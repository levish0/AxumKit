use crate::entity::common::OAuthProvider;
use crate::entity::user_oauth_connections::ActiveModel as OAuthConnectionActiveModel;
use crate::errors::errors::Errors;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use tracing::error;
use uuid::Uuid;

/// OAuth 연결 생성
pub async fn repository_create_oauth_connection<C>(
    conn: &C,
    user_id: &Uuid,
    provider: OAuthProvider,
    provider_user_id: &str,
) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    let oauth_connection = OAuthConnectionActiveModel {
        id: Default::default(),
        user_id: Set(*user_id),
        provider: Set(provider),
        provider_user_id: Set(provider_user_id.to_string()),
        created_at: Set(Utc::now().into()),
    };

    oauth_connection.insert(conn).await.map_err(|e| {
        error!("Failed to create OAuth connection: {:?}", e);
        Errors::DatabaseError(e.to_string())
    })?;

    Ok(())
}
