use crate::repository::user::repository_find_user_by_email;
use crate::service::auth::session::SessionService;
use crate::service::auth::totp::TotpTempToken;
use axumkit_dto::auth::request::LoginRequest;
use axumkit_errors::errors::{Errors, ServiceResult};

use crate::utils::crypto::password::verify_password;
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;

/// 로그인 결과: 세션 생성 또는 TOTP 필요
pub enum LoginResult {
    /// TOTP 없음: 세션 ID 반환
    SessionCreated {
        session_id: String,
        remember_me: bool,
    },
    /// TOTP 필요: 임시 토큰 반환
    TotpRequired(String),
}

pub async fn service_login(
    conn: &DatabaseConnection,
    redis: &ConnectionManager,
    payload: LoginRequest,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<LoginResult> {
    // 사용자 검증
    let user = repository_find_user_by_email(conn, payload.email.clone())
        .await?
        .ok_or(Errors::UserNotFound)?;

    // 비밀번호 검증
    let password_hash = user.password.ok_or(Errors::UserPasswordNotSet)?;
    verify_password(&payload.password, &password_hash)?;

    // TOTP 활성화 확인
    if user.totp_enabled_at.is_some() {
        // TOTP 필요: 임시 토큰 생성
        let temp_token =
            TotpTempToken::create(redis, user.id, user_agent, ip_address, payload.remember_me)
                .await?;

        return Ok(LoginResult::TotpRequired(temp_token.token));
    }

    // TOTP 없음: 바로 세션 생성
    let session =
        SessionService::create_session(redis, user.id.to_string(), user_agent, ip_address).await?;

    Ok(LoginResult::SessionCreated {
        session_id: session.session_id,
        remember_me: payload.remember_me,
    })
}
