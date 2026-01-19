use crate::repository::user::{
    UserUpdateParams, repository_find_user_by_email, repository_update_user,
};
use crate::service::auth::change_email::EmailChangeData;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use tracing::info;
use uuid::Uuid;

/// 이메일 변경을 확인합니다.
pub async fn service_confirm_email_change<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    token: &str,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    // 1. Redis에서 토큰 검증 (get_del로 일회용)
    let token_key = format!("email_change:{}", token);
    let mut redis_mut = redis_conn.clone();

    let token_json: Option<String> = redis_mut
        .get_del(&token_key)
        .await
        .map_err(|e| Errors::SysInternalError(format!("Redis error: {}", e)))?;

    let token_data = token_json.ok_or(Errors::TokenInvalidEmailChange)?;

    let change_data: EmailChangeData =
        serde_json::from_str(&token_data).map_err(|_| Errors::TokenInvalidEmailChange)?;

    // 2. user_id 파싱
    let user_id =
        Uuid::parse_str(&change_data.user_id).map_err(|_| Errors::TokenInvalidEmailChange)?;

    // 3. 이메일 중복 체크 (토큰 발급 후 다른 사용자가 해당 이메일을 사용했을 수 있음)
    if let Some(existing) =
        repository_find_user_by_email(conn, change_data.new_email.clone()).await?
        && existing.id != user_id
    {
        return Err(Errors::UserEmailAlreadyExists);
    }

    // 4. 이메일 업데이트 (verified_at도 현재 시간으로 설정 - 이메일 인증 완료로 간주)
    repository_update_user(
        conn,
        user_id,
        UserUpdateParams {
            email: Some(change_data.new_email.clone()),
            verified_at: Some(Some(chrono::Utc::now())),
            ..Default::default()
        },
    )
    .await?;

    info!(
        "Email changed successfully for user {} to {}",
        user_id, change_data.new_email
    );

    Ok(())
}
