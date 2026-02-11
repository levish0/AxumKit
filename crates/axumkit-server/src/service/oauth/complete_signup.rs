use crate::connection::r2_conn::R2Client;
use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::create_oauth_user::repository_create_oauth_user;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_email::repository_find_user_by_email;
use crate::repository::user::find_by_handle::repository_find_user_by_handle;
use crate::service::auth::session::SessionService;
use crate::service::oauth::download_profile_image::download_and_upload_profile_image;
use crate::service::oauth::types::PendingSignupData;
use axumkit_constants::oauth_pending_key;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use reqwest::Client as HttpClient;
use sea_orm::{ConnectionTrait, TransactionSession, TransactionTrait};

/// OAuth pending signup을 완료하고 세션을 생성합니다.
///
/// # Arguments
/// * `conn` - 데이터베이스 연결
/// * `redis_conn` - Redis 연결
/// * `pending_token` - pending signup 토큰
/// * `handle` - 사용자 핸들
/// * `user_agent` - User-Agent 헤더
/// * `ip_address` - IP 주소
///
/// # Returns
/// * `String` - Session ID
pub async fn service_complete_signup<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &HttpClient,
    r2_client: &R2Client,
    pending_token: &str,
    handle: &str,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<String>
where
    C: ConnectionTrait + TransactionTrait,
{
    // 1. Redis에서 pending 데이터 조회 + 삭제 (get_del로 1회용)
    let pending_key = oauth_pending_key(pending_token);
    let mut redis_mut = redis_conn.clone();
    let pending_json: Option<String> = redis_mut
        .get_del(&pending_key)
        .await
        .map_err(|e| Errors::SysInternalError(format!("Redis error: {}", e)))?;

    let pending_data: PendingSignupData = match pending_json {
        Some(json) => serde_json::from_str(&json).map_err(|_| Errors::UserInvalidToken)?,
        None => return Err(Errors::UserTokenExpired),
    };

    // 2. 트랜잭션 시작 - 모든 체크를 트랜잭션 안에서 수행 (race condition 방지)
    let txn = conn.begin().await?;

    // 2.1 OAuth 연결이 이미 있는지 확인
    if repository_find_user_by_oauth(
        &txn,
        pending_data.provider.clone(),
        &pending_data.provider_user_id,
    )
    .await?
    .is_some()
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 2.2 이메일이 이미 존재하는지 확인
    if repository_find_user_by_email(&txn, pending_data.email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // 2.3 handle 중복 확인
    if repository_find_user_by_handle(&txn, handle.to_string())
        .await?
        .is_some()
    {
        return Err(Errors::UserHandleAlreadyExists);
    }

    // 3. 프로필 이미지 다운로드 → R2 업로드
    let profile_image_key = match pending_data.profile_image {
        Some(ref url) => download_and_upload_profile_image(http_client, r2_client, url).await,
        None => None,
    };

    // 4. 사용자 생성
    let new_user = repository_create_oauth_user(
        &txn,
        &pending_data.email,
        &pending_data.display_name,
        handle,
        profile_image_key,
    )
    .await?;

    // 5. OAuth 연결 생성
    repository_create_oauth_connection(
        &txn,
        &new_user.id,
        pending_data.provider,
        &pending_data.provider_user_id,
    )
    .await?;

    txn.commit().await?;

    // 6. 세션 생성
    let session =
        SessionService::create_session(redis_conn, new_user.id.to_string(), user_agent, ip_address)
            .await?;

    Ok(session.session_id)
}
