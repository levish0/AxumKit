use super::GithubProvider;
use crate::service::oauth::generate_oauth_url::service_generate_oauth_url;
use axumkit_dto::oauth::request::OAuthAuthorizeFlow;
use axumkit_dto::oauth::response::OAuthUrlResponse;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::ServiceResult;
use redis::aio::ConnectionManager;

/// Generates a GitHub OAuth authorization URL and stores the state in Redis.
pub async fn service_generate_github_oauth_url(
    redis_conn: &ConnectionManager,
    anonymous_user_id: &str,
    flow: OAuthAuthorizeFlow,
) -> ServiceResult<OAuthUrlResponse> {
    service_generate_oauth_url::<GithubProvider>(
        redis_conn,
        anonymous_user_id,
        flow,
        OAuthProvider::Github,
    )
    .await
}
