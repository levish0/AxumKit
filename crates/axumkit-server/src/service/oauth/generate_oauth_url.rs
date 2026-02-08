use crate::service::oauth::provider::client::generate_auth_url;
use crate::service::oauth::provider::config::OAuthProviderConfig;
use crate::service::oauth::types::OAuthStateData;
use crate::utils::redis_cache::set_json_with_ttl;
use axumkit_constants::oauth_state_key;
use axumkit_dto::oauth::response::OAuthUrlResponse;
use axumkit_errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use uuid::Uuid;

/// OAuth 인증 URL을 생성하고 state를 Redis에 저장합니다.
pub async fn service_generate_oauth_url<P: OAuthProviderConfig>(
    redis_conn: &ConnectionManager,
) -> ServiceResult<OAuthUrlResponse> {
    // 1. State 생성
    let state = Uuid::now_v7().to_string();

    // 2. OAuth 인증 URL 생성 (PKCE 포함)
    let (auth_url, _state, pkce_verifier) = generate_auth_url::<P>(state.clone())?;

    // 3. State와 PKCE verifier를 Redis에 저장
    let state_data = OAuthStateData { pkce_verifier };
    let state_key = oauth_state_key(&state);
    set_json_with_ttl(
        redis_conn,
        &state_key,
        &state_data,
        axumkit_constants::OAUTH_STATE_TTL_SECONDS,
    )
    .await?;

    Ok(OAuthUrlResponse { auth_url })
}
