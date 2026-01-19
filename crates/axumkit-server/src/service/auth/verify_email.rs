use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use axumkit_errors::errors::{Errors, ServiceResult};
use chrono::Utc;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailVerificationData {
    pub user_id: String,
    pub email: String,
}

/// 이메일 인증을 처리합니다.
///
/// # Arguments
/// * `conn` - 데이터베이스 연결
/// * `redis_conn` - Redis 연결
/// * `token` - 이메일 인증 토큰
///
/// # Returns
/// * `()` - 성공 시
pub async fn service_verify_email<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    token: &str,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    // 1. Redis에서 토큰 검증 (get_del로 일회용)
    let token_key = format!("email_verification:{}", token);
    let mut redis_mut = redis_conn.clone();

    let token_json: Option<String> = redis_mut
        .get_del(&token_key)
        .await
        .map_err(|e| Errors::SysInternalError(format!("Redis error: {}", e)))?;

    let token_data = token_json.ok_or(Errors::TokenInvalidVerification)?;

    let verification_data: EmailVerificationData =
        serde_json::from_str(&token_data).map_err(|_| Errors::TokenInvalidVerification)?;

    // 2. user_id 파싱
    let user_id = Uuid::parse_str(&verification_data.user_id)
        .map_err(|_| Errors::TokenInvalidVerification)?;

    // 3. 사용자 조회
    let user = repository_get_user_by_id(conn, user_id).await?;

    // 4. 이미 인증된 사용자인지 확인
    if user.verified_at.is_some() {
        return Err(Errors::EmailAlreadyVerified);
    }

    // 5. 이메일 주소 일치 확인
    if user.email != verification_data.email {
        return Err(Errors::TokenEmailMismatch);
    }

    // 6. verified_at 업데이트
    repository_update_user(
        conn,
        user_id,
        UserUpdateParams {
            verified_at: Some(Some(Utc::now())),
            ..Default::default()
        },
    )
    .await?;

    Ok(())
}
