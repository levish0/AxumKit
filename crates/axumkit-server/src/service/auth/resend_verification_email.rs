use crate::bridge::worker_client;
use crate::repository::user::repository_get_user_by_id;
use crate::service::auth::verify_email::EmailVerificationData;
use crate::state::WorkerClient;
use crate::utils::crypto::token::generate_secure_token;
use crate::utils::redis_cache::set_json_with_ttl;
use axumkit_config::ServerConfig;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

/// 이메일 인증 메일을 재발송합니다.
///
/// # Arguments
/// * `conn` - 데이터베이스 연결
/// * `redis_conn` - Redis 연결
/// * `worker` - Worker Redis 연결
/// * `user_id` - 사용자 ID
///
/// # Returns
/// * `()` - 성공 시
pub async fn service_resend_verification_email<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    user_id: Uuid,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    let config = ServerConfig::get();

    // 1. 사용자 조회
    let user = repository_get_user_by_id(conn, user_id).await?;

    // 2. 이미 인증된 사용자인지 확인
    if user.verified_at.is_some() {
        return Err(Errors::EmailAlreadyVerified);
    }

    // 3. OAuth 전용 사용자인지 확인 (password가 없으면 OAuth 사용자)
    if user.password.is_none() {
        return Err(Errors::UserPasswordNotSet);
    }

    // 4. 새 토큰 생성 (암호학적으로 안전한 랜덤 토큰)
    let token = generate_secure_token();
    let token_key = format!("email_verification:{}", token);

    let verification_data = EmailVerificationData {
        user_id: user.id.to_string(),
        email: user.email.clone(),
    };

    // 5. Redis에 토큰 저장 (분 단위 → 초 단위 변환)
    let ttl_seconds = (config.auth_email_verification_token_expire_time * 60) as u64;
    set_json_with_ttl(redis_conn, &token_key, &verification_data, ttl_seconds).await?;

    // 6. Worker 서비스에 이메일 발송 요청
    worker_client::send_verification_email(
        worker,
        &user.email,
        &user.handle,
        &token,
        config.auth_email_verification_token_expire_time as u64,
    )
    .await?;

    Ok(())
}
