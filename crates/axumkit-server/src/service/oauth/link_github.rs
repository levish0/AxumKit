use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::find_oauth_connection::repository_find_oauth_connection;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::service::oauth::provider::github::{exchange_github_code, fetch_github_user_info};
use crate::service::oauth::types::OAuthStateData;
use axumkit_config::ServerConfig;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

/// GitHub OAuth를 기존 계정에 연결합니다.
///
/// # Arguments
/// * `conn` - 데이터베이스 연결
/// * `redis_conn` - Redis 연결
/// * `http_client` - HTTP 클라이언트
/// * `user_id` - 연결할 사용자 ID
/// * `code` - GitHub로부터 받은 authorization code
/// * `state` - CSRF 방지용 state
#[allow(clippy::too_many_arguments)]
pub async fn service_link_github_oauth<C>(
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
    let config = ServerConfig::get();

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
    let access_token = exchange_github_code(
        http_client,
        &config.github_client_id,
        &config.github_client_secret,
        &config.github_redirect_uri,
        code,
        &state_data.pkce_verifier,
    )
    .await?;

    // 3. Access token으로 사용자 정보 가져오기
    let user_info = fetch_github_user_info(http_client, &access_token).await?;

    // 4. 이미 다른 계정에 연결되어 있는지 확인
    if let Some(_existing_user) =
        repository_find_user_by_oauth(conn, OAuthProvider::Github, &user_info.id.to_string())
            .await?
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 5. 현재 유저에게 이미 GitHub이 연결되어 있는지 확인
    if repository_find_oauth_connection(conn, user_id, OAuthProvider::Github)
        .await?
        .is_some()
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 6. OAuth 연결 생성
    repository_create_oauth_connection(
        conn,
        &user_id,
        OAuthProvider::Github,
        &user_info.id.to_string(),
    )
    .await?;

    Ok(())
}
