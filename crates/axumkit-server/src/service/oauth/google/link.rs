use super::{GoogleProvider, fetch_google_user_info};
use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::find_oauth_connection::repository_find_oauth_connection;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::service::oauth::provider::client::exchange_code;
use crate::service::oauth::types::OAuthStateData;
use axumkit_constants::oauth_state_key;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

/// Google OAuth를 기존 계정에 연결합니다.
pub async fn service_link_google_oauth<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    user_id: Uuid,
    code: &str,
    state: &str,
) -> ServiceResult<()>
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

    // 4. 이미 다른 계정에 연결되어 있는지 확인
    if repository_find_user_by_oauth(conn, OAuthProvider::Google, &user_info.id)
        .await?
        .is_some()
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 5. 현재 유저에게 이미 Google이 연결되어 있는지 확인
    if repository_find_oauth_connection(conn, user_id, OAuthProvider::Google)
        .await?
        .is_some()
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 6. OAuth 연결 생성
    repository_create_oauth_connection(conn, &user_id, OAuthProvider::Google, &user_info.id)
        .await?;

    Ok(())
}
