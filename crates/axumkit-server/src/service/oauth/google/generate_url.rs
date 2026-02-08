use super::GoogleProvider;
use crate::service::oauth::generate_oauth_url::service_generate_oauth_url;
use redis::aio::ConnectionManager;
use axumkit_dto::oauth::response::OAuthUrlResponse;
use axumkit_errors::errors::ServiceResult;

/// Google OAuth 인증 URL을 생성하고 state를 Redis에 저장합니다.
pub async fn service_generate_google_oauth_url(
    redis_conn: &ConnectionManager,
) -> ServiceResult<OAuthUrlResponse> {
    service_generate_oauth_url::<GoogleProvider>(redis_conn).await
}
