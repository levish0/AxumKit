use super::common::verify_totp_code;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::service::auth::session::SessionService;
use crate::service::auth::totp::TotpTempToken;
use crate::utils::crypto::backup_code::verify_backup_code;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager as RedisClient;
use sea_orm::ConnectionTrait;

/// TOTP 검증 결과
pub struct TotpVerifyResult {
    pub session_id: String,
    pub remember_me: bool,
}

/// TOTP 검증 (로그인 2단계): temp_token + code → session 생성
pub async fn service_totp_verify<C>(
    conn: &C,
    redis: &RedisClient,
    temp_token: &str,
    code: &str,
) -> ServiceResult<TotpVerifyResult>
where
    C: ConnectionTrait,
{
    // 임시 토큰 조회 및 삭제 (일회용)
    let token_data = TotpTempToken::get_and_delete(redis, temp_token)
        .await?
        .ok_or(Errors::TotpTempTokenInvalid)?;

    // 사용자 조회
    let user = repository_get_user_by_id(conn, token_data.user_id).await?;

    // TOTP가 활성화되어 있어야 함
    if user.totp_enabled_at.is_none() {
        return Err(Errors::TotpNotEnabled);
    }

    let secret_base32 = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;
    let backup_codes = user.totp_backup_codes.clone().unwrap_or_default();

    // 코드 길이로 TOTP vs 백업 코드 구분
    if code.len() == 6 {
        // TOTP 코드 검증
        if !verify_totp_code(&secret_base32, &user.email, code)? {
            return Err(Errors::TotpInvalidCode);
        }
    } else if code.len() == 8 {
        // 백업 코드 검증
        if backup_codes.is_empty() {
            return Err(Errors::TotpBackupCodeExhausted);
        }

        // 해시 비교로 백업 코드 검증
        if let Some(idx) = verify_backup_code(code, &backup_codes) {
            // 사용된 백업 코드 제거
            let mut new_codes = backup_codes.clone();
            new_codes.remove(idx);

            repository_update_user(
                conn,
                token_data.user_id,
                UserUpdateParams {
                    totp_backup_codes: Some(Some(new_codes)),
                    ..Default::default()
                },
            )
            .await?;
        } else {
            return Err(Errors::TotpInvalidCode);
        }
    } else {
        return Err(Errors::TotpInvalidCode);
    }

    // 세션 생성
    let session = SessionService::create_session(
        redis,
        token_data.user_id.to_string(),
        token_data.user_agent,
        token_data.ip_address,
    )
    .await?;

    Ok(TotpVerifyResult {
        session_id: session.session_id,
        remember_me: token_data.remember_me,
    })
}
