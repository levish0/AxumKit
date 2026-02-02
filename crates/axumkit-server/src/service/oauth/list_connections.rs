use crate::repository::oauth::list_oauth_connections::repository_list_oauth_connections_by_user_id;
use sea_orm::ConnectionTrait;
use uuid::Uuid;
use axumkit_dto::oauth::response::{OAuthConnectionListResponse, OAuthConnectionResponse};
use axumkit_errors::errors::ServiceResult;

/// 사용자의 모든 OAuth 연결 목록을 조회합니다.
pub async fn service_list_oauth_connections<C>(
    conn: &C,
    user_id: Uuid,
) -> ServiceResult<OAuthConnectionListResponse>
where
    C: ConnectionTrait,
{
    let connections = repository_list_oauth_connections_by_user_id(conn, user_id).await?;

    Ok(OAuthConnectionListResponse {
        connections: connections
            .into_iter()
            .map(OAuthConnectionResponse::from)
            .collect(),
    })
}
