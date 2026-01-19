use super::common::{generate_backup_codes, verify_totp_code};
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::crypto::backup_code::hash_backup_codes;
use axumkit_dto::auth::response::TotpBackupCodesResponse;
use axumkit_errors::errors::{Errors, ServiceResult};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

/// 백업 코드 재생성: 현재 TOTP 코드 검증 후 새 백업 코드 생성
pub async fn service_regenerate_backup_codes<C>(
    conn: &C,
    user_id: Uuid,
    email: &str,
    code: &str,
) -> ServiceResult<TotpBackupCodesResponse>
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

    // TOTP 코드 검증 (백업 코드 재생성은 반드시 TOTP 코드로만)
    if !verify_totp_code(&secret_base32, email, code)? {
        return Err(Errors::TotpInvalidCode);
    }

    // 새 백업 코드 생성 (평문)
    let backup_codes = generate_backup_codes();
    // 해시하여 DB에 저장
    let hashed_codes = hash_backup_codes(&backup_codes);

    // DB 업데이트
    repository_update_user(
        conn,
        user_id,
        UserUpdateParams {
            totp_backup_codes: Some(Some(hashed_codes)),
            ..Default::default()
        },
    )
    .await?;

    // 평문 백업 코드 반환 (사용자가 저장해야 함)
    Ok(TotpBackupCodesResponse { backup_codes })
}
