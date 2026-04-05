use crate::service::oauth::provider::client::generate_auth_url;
use crate::service::oauth::provider::config::OAuthProviderConfig;
use crate::service::oauth::types::OAuthStateData;
use crate::utils::redis_cache::store_json_for_token_with_ttl;
use axumkit_dto::oauth::request::OAuthAuthorizeFlow;
use axumkit_dto::oauth::response::OAuthUrlResponse;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use uuid::Uuid;

/// Generates an OAuth authorization URL and stores the state in Redis.
pub async fn service_generate_oauth_url<P: OAuthProviderConfig>(
    redis_conn: &ConnectionManager,
    anonymous_user_id: &str,
    flow: OAuthAuthorizeFlow,
    provider: OAuthProvider,
) -> ServiceResult<OAuthUrlResponse> {
    // 1. Generate state
    let state = Uuid::now_v7().to_string();

    // 2. Generate OAuth authorization URL (with PKCE)
    let (auth_url, _state, pkce_verifier) = generate_auth_url::<P>(state.clone())?;

    // 3. Store state and PKCE verifier in Redis
    let state_data = OAuthStateData {
        pkce_verifier,
        flow,
        provider,
        anonymous_user_id: anonymous_user_id.to_string(),
    };
    store_json_for_token_with_ttl(
        redis_conn,
        &state,
        axumkit_constants::oauth_state_key,
        &state_data,
        axumkit_constants::OAUTH_STATE_TTL_SECONDS,
    )
    .await?;

    Ok(OAuthUrlResponse { auth_url })
}
