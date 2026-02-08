use super::{GoogleProvider, fetch_google_user_info};
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_email::repository_find_user_by_email;
use crate::service::auth::session::SessionService;
use crate::service::oauth::provider::client::exchange_code;
use crate::service::oauth::types::OAuthStateData;
use crate::service::oauth::types::PendingSignupData;
use crate::utils::redis_cache::set_json_with_ttl;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use axumkit_config::ServerConfig;
use axumkit_constants::{oauth_pending_key, oauth_state_key};
use axumkit_dto::oauth::internal::SignInResult;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::{Errors, ServiceResult};

/// Google OAuth 로그인을 처리합니다.
///
/// - 기존 사용자: 세션 생성 후 Success 반환
/// - 신규 사용자: PendingSignup 반환 (complete-signup으로 가입 완료 필요)
pub async fn service_google_sign_in<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    code: &str,
    state: &str,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<SignInResult>
where
    C: ConnectionTrait,
{
    // 1. Redis에서 state 검증 및 PKCE verifier 조회 (get_del로 1회용)
    let state_key = oauth_state_key(state);
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
    let access_token =
        exchange_code::<GoogleProvider>(http_client, code, &state_data.pkce_verifier).await?;

    // 3. Access token으로 사용자 정보 가져오기
    let user_info = fetch_google_user_info(http_client, &access_token).await?;

    // 3-1. 이메일 검증 여부 확인
    if !user_info.verified_email {
        return Err(Errors::OauthEmailNotVerified);
    }

    // 4. 기존 OAuth 연결 확인
    if let Some(existing_user) =
        repository_find_user_by_oauth(conn, OAuthProvider::Google, &user_info.id).await?
    {
        // 기존 사용자 - 세션 생성 후 Success 반환
        let session = SessionService::create_session(
            redis_conn,
            existing_user.id.to_string(),
            user_agent,
            ip_address,
        )
        .await?;

        return Ok(SignInResult::Success(session.session_id));
    }

    // 5. 신규 사용자 - 이메일 중복 확인
    if repository_find_user_by_email(conn, user_info.email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // 6. 신규 사용자 - pending signup 데이터를 Redis에 저장
    let config = ServerConfig::get();
    let pending_token = uuid::Uuid::new_v4().to_string();
    let pending_data = PendingSignupData {
        provider: OAuthProvider::Google,
        provider_user_id: user_info.id,
        email: user_info.email.clone(),
        display_name: user_info.name.clone(),
        profile_image: Some(user_info.picture),
    };

    let pending_key = oauth_pending_key(&pending_token);
    let ttl_seconds = (config.oauth_pending_signup_ttl_minutes * 60) as u64;
    set_json_with_ttl(redis_conn, &pending_key, &pending_data, ttl_seconds).await?;

    Ok(SignInResult::PendingSignup {
        pending_token,
        email: user_info.email,
        display_name: user_info.name,
    })
}
