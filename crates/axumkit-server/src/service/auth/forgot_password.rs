use crate::bridge::worker_client;
use crate::repository::user::repository_find_user_by_email;
use crate::state::WorkerClient;
use crate::utils::crypto::token::generate_secure_token;
use crate::utils::redis_cache::set_json_with_ttl;
use axumkit_config::ServerConfig;
use axumkit_errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Redis에 저장되는 비밀번호 재설정 토큰 데이터
#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordResetData {
    pub user_id: String,
}

/// 비밀번호 재설정 이메일을 발송합니다.
///
/// 보안: 이메일 존재 여부와 관계없이 항상 성공을 반환합니다.
pub async fn service_forgot_password<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    email: &str,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    let config = ServerConfig::get();

    // 1. 이메일로 사용자 조회
    let user = repository_find_user_by_email(conn, email.to_string()).await?;

    // 2. 사용자가 없으면 조용히 반환 (이메일 존재 여부 노출 방지)
    let user = match user {
        Some(u) => u,
        None => {
            info!("Password reset requested for non-existent email: {}", email);
            return Ok(());
        }
    };

    // 3. 비밀번호가 설정되지 않은 사용자는 조용히 반환
    if user.password.is_none() {
        info!(
            "Password reset requested for user without password: {}",
            email
        );
        return Ok(());
    }

    // 4. 토큰 생성
    let token = generate_secure_token();
    let token_key = format!("password_reset:{}", token);

    let reset_data = PasswordResetData {
        user_id: user.id.to_string(),
    };

    // 5. Redis에 토큰 저장 (분 단위 → 초 단위 변환)
    let ttl_seconds = (config.auth_password_reset_token_expire_time * 60) as u64;
    set_json_with_ttl(redis_conn, &token_key, &reset_data, ttl_seconds).await?;

    // 6. Worker 서비스에 이메일 발송 요청
    worker_client::send_password_reset_email(
        worker,
        &user.email,
        &user.handle,
        &token,
        config.auth_password_reset_token_expire_time as u64,
    )
    .await?;

    info!("Password reset email sent to: {}", email);

    Ok(())
}
