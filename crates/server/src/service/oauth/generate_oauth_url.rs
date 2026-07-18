use crate::service::oauth::provider::client::generate_auth_url;
use crate::service::oauth::provider::config::OAuthProviderConfig;
use crate::service::oauth::types::OAuthStateData;
use crate::utils::crypto::token::{generate_secure_token, hash_token};
use crate::utils::redis_cache::store_json_for_token_with_ttl;
use dto::oauth::request::OAuthAuthorizeFlow;
use dto::oauth::response::OAuthUrlResponse;
use entity::common::OAuthProvider;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;

/// Generates an OAuth authorization URL and stores the state in Redis.
pub async fn service_generate_oauth_url<P: OAuthProviderConfig>(
    redis_conn: &ConnectionManager,
    anonymous_user_id: &str,
    flow: OAuthAuthorizeFlow,
    provider: OAuthProvider,
) -> ServiceResult<OAuthUrlResponse> {
    // 1. Generate state — 256-bit CSPRNG token (not a time-ordered UUID, which would leak
    //    issuance time and carry less entropy) matching the codebase's token standard.
    let state = generate_secure_token();

    // 2. Generate OAuth authorization URL (with PKCE)
    let (auth_url, _state, pkce_verifier) = generate_auth_url::<P>(state.clone())?;

    // 3. Store state and PKCE verifier in Redis
    let state_data = OAuthStateData {
        pkce_verifier,
        flow,
        provider,
        anonymous_user_id: anonymous_user_id.to_string(),
    };
    // Key by the hashed state (hash-at-rest, like every other short-lived
    // credential): the payload carries the plaintext PKCE verifier, so a Redis
    // snapshot/leak must not expose live state values alongside their verifiers.
    // The raw state only travels in the authorization URL.
    store_json_for_token_with_ttl(
        redis_conn,
        &state,
        |token| constants::oauth_state_key(&hash_token(token)),
        &state_data,
        constants::OAUTH_STATE_TTL_SECONDS,
    )
    .await?;

    Ok(OAuthUrlResponse { auth_url })
}
