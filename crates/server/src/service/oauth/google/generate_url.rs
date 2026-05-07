use super::GoogleProvider;
use crate::service::oauth::generate_oauth_url::service_generate_oauth_url;
use dto::oauth::request::OAuthAuthorizeFlow;
use dto::oauth::response::OAuthUrlResponse;
use entity::common::OAuthProvider;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;

/// Generates a Google OAuth authorization URL and stores the state in Redis.
pub async fn service_generate_google_oauth_url(
    redis_conn: &ConnectionManager,
    anonymous_user_id: &str,
    flow: OAuthAuthorizeFlow,
) -> ServiceResult<OAuthUrlResponse> {
    service_generate_oauth_url::<GoogleProvider>(
        redis_conn,
        anonymous_user_id,
        flow,
        OAuthProvider::Google,
    )
    .await
}
