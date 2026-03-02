use super::{GithubProvider, fetch_github_user_info};
use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::find_oauth_connection::repository_find_oauth_connection;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::service::oauth::provider::client::exchange_code;
use crate::service::oauth::types::OAuthStateData;
use crate::utils::redis_cache::get_json_and_delete;
use axumkit_constants::oauth_state_key;
use axumkit_dto::oauth::request::OAuthAuthorizeFlow;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

/// GitHub OAuth를 기존 계정에 연결합니다.
pub async fn service_link_github_oauth(
    conn: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    user_id: Uuid,
    code: &str,
    state: &str,
    anonymous_user_id: &str,
) -> ServiceResult<()> {
    // 1. Redis에서 state 검증 및 PKCE verifier 조회 (get_del로 1회용)
    let state_key = oauth_state_key(state);
    let state_data: OAuthStateData = get_json_and_delete(
        redis_conn,
        &state_key,
        || Errors::OauthInvalidState,
        |_| Errors::OauthInvalidState,
    )
    .await?;

    // 2. Authorization code를 access token으로 교환
    if state_data.provider != OAuthProvider::Github
        || state_data.flow != OAuthAuthorizeFlow::Link
        || state_data.anonymous_user_id != anonymous_user_id
    {
        return Err(Errors::OauthInvalidState);
    }

    let access_token =
        exchange_code::<GithubProvider>(http_client, code, &state_data.pkce_verifier).await?;

    // 3. Access token으로 사용자 정보 가져오기
    let user_info = fetch_github_user_info(http_client, &access_token).await?;

    let txn = conn.begin().await?;

    // 4. 이미 다른 계정에 연결되어 있는지 확인
    if repository_find_user_by_oauth(&txn, OAuthProvider::Github, &user_info.id.to_string())
        .await?
        .is_some()
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 5. 현재 유저에게 이미 GitHub이 연결되어 있는지 확인
    if repository_find_oauth_connection(&txn, user_id, OAuthProvider::Github)
        .await?
        .is_some()
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 6. OAuth 연결 생성
    repository_create_oauth_connection(
        &txn,
        &user_id,
        OAuthProvider::Github,
        &user_info.id.to_string(),
    )
    .await?;

    txn.commit().await?;

    Ok(())
}
