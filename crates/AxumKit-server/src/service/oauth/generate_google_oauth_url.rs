use crate::config::db_config::DbConfig;
use crate::dto::oauth::internal::oauth_state_data::OAuthStateData;
use crate::dto::oauth::response::oauth_url::OAuthUrlResponse;
use crate::errors::errors::Errors;
use crate::service::oauth::provider::google::client::generate_google_auth_url;
use crate::utils::redis_cache::set_json_with_ttl;
use redis::aio::ConnectionManager;
use uuid::Uuid;

/// Google OAuth 인증 URL을 생성하고 state를 Redis에 저장합니다.
///
/// # Arguments
/// * `redis_conn` - Redis 연결
///
/// # Returns
/// * `OAuthUrlResponse` - 인증 URL
pub async fn service_generate_google_oauth_url(
    redis_conn: &ConnectionManager,
) -> Result<OAuthUrlResponse, Errors> {
    let config = DbConfig::get();

    // 1. State 생성
    let state = Uuid::new_v4().to_string();

    // 2. OAuth 인증 URL 생성 (PKCE 포함)
    let (auth_url, _state, pkce_verifier) = generate_google_auth_url(
        &config.google_client_id,
        &config.google_client_secret,
        &config.google_redirect_uri,
        state.clone(),
    )?;

    // 3. State와 PKCE verifier를 Redis에 저장
    let state_data = OAuthStateData { pkce_verifier };

    let state_key = format!("oauth:state:{}", state);

    // 5분 TTL로 Redis에 저장
    set_json_with_ttl(redis_conn, &state_key, &state_data, 300).await?;

    Ok(OAuthUrlResponse { auth_url })
}
