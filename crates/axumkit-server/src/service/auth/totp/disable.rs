use super::common::verify_totp_code;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::crypto::backup_code::verify_backup_code;
use axumkit_errors::errors::{Errors, ServiceResult};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

/// TOTP 비활성화: 현재 코드 검증 후 모든 TOTP 필드 초기화
pub async fn service_totp_disable<C>(
    conn: &C,
    user_id: Uuid,
    email: &str,
    code: &str,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    // 사용자 조회
    let user = repository_get_user_by_id(conn, user_id).await?;

    // TOTP가 활성화되어 있어야 함
    if user.totp_enabled_at.is_none() {
        return Err(Errors::TotpNotEnabled);
    }

    let secret_base32 = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;
    let backup_codes = user.totp_backup_codes.clone().unwrap_or_default();

    // 코드 검증 (TOTP 6자리 또는 백업 코드 8자리)
    if code.len() == 6 {
        if !verify_totp_code(&secret_base32, email, code)? {
            return Err(Errors::TotpInvalidCode);
        }
    } else if code.len() == 8 {
        // 해시 비교로 백업 코드 검증
        if verify_backup_code(code, &backup_codes).is_none() {
            return Err(Errors::TotpInvalidCode);
        }
    } else {
        return Err(Errors::TotpInvalidCode);
    }

    // TOTP 비활성화 (모든 필드 초기화)
    repository_update_user(
        conn,
        user_id,
        UserUpdateParams {
            totp_secret: Some(None),
            totp_enabled_at: Some(None),
            totp_backup_codes: Some(None),
            ..Default::default()
        },
    )
    .await?;

    Ok(())
}
