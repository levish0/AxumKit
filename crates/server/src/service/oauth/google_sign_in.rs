use crate::config::server_config::DbConfig;
use crate::dto::oauth::internal::oauth_state_data::OAuthStateData;
use crate::entity::common::OAuthProvider;
use crate::errors::errors::Errors;
use crate::service::auth::session::SessionService;
use crate::service::oauth::find_or_create_oauth_user::service_find_or_create_oauth_user;
use crate::service::oauth::provider::google::client::{
    exchange_google_code, fetch_google_user_info,
};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sea_orm::{ConnectionTrait, TransactionSession, TransactionTrait};

/// Google OAuth 로그인을 처리하고 세션을 생성합니다.
///
/// # Arguments
/// * `db_conn` - 데이터베이스 연결
/// * `redis_conn` - Redis 연결
/// * `http_client` - HTTP 클라이언트
/// * `code` - Google로부터 받은 authorization code
/// * `state` - CSRF 방지용 state
/// * `handle` - 사용자 핸들 (신규 사용자 생성 시)
/// * `user_agent` - User-Agent 헤더
/// * `ip_address` - IP 주소
///
/// # Returns
/// * `String` - Session ID
pub async fn service_google_sign_in<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    code: &str,
    state: &str,
    handle: Option<String>,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<String, Errors>
where
    C: ConnectionTrait + TransactionTrait,
{
    let config = DbConfig::get();

    // 1. Redis에서 state 검증 및 PKCE verifier 조회 (get_del로 1회용)
    let state_key = format!("oauth:state:{}", state);
    let mut redis_mut = redis_conn.clone();
    let state_json: Option<String> = redis_mut
        .get_del(&state_key)
        .await
        .map_err(|e| Errors::SysInternalError(format!("Redis error: {}", e)))?;

    let state_data = match state_json {
        Some(json) => {
            serde_json::from_str::<OAuthStateData>(&json).map_err(|_| Errors::OauthInvalidState)?
        }
        None => return Err(Errors::OauthInvalidState),
    };

    // 2. Authorization code를 access token으로 교환
    let access_token = exchange_google_code(
        http_client,
        &config.google_client_id,
        &config.google_client_secret,
        &config.google_redirect_uri,
        code,
        &state_data.pkce_verifier,
    )
    .await?;

    // 3. Access token으로 사용자 정보 가져오기
    let user_info = fetch_google_user_info(http_client, &access_token).await?;

    // 4. 트랜잭션 시작 - 사용자 찾기/생성
    let txn = conn.begin().await?;

    let oauth_user_result = service_find_or_create_oauth_user(
        &txn,
        OAuthProvider::Google,
        &user_info.id,
        &user_info.email,
        &user_info.name,
        handle.as_deref(),
        Some(user_info.picture),
    )
    .await?;

    txn.commit().await?;

    // 5. 세션 생성
    let session = SessionService::create_session(
        redis_conn,
        oauth_user_result.user.id.to_string(),
        user_agent,
        ip_address,
    )
    .await?;

    Ok(session.session_id)
}
