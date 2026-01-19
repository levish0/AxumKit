use super::common::{generate_backup_codes, verify_totp_code};
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::crypto::backup_code::hash_backup_codes;
use axumkit_dto::auth::response::TotpEnableResponse;
use axumkit_errors::errors::{Errors, ServiceResult};
use chrono::Utc;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

/// TOTP 활성화: 첫 코드 검증 후 활성화 + 백업 코드 생성
pub async fn service_totp_enable<C>(
    conn: &C,
    user_id: Uuid,
    email: &str,
    code: &str,
) -> ServiceResult<TotpEnableResponse>
where
    C: ConnectionTrait,
{
    // 사용자 조회
    let user = repository_get_user_by_id(conn, user_id).await?;

    // 이미 TOTP 활성화된 경우
    if user.totp_enabled_at.is_some() {
        return Err(Errors::TotpAlreadyEnabled);
    }

    // Secret이 없는 경우 (setup 안 함)
    let secret_base32 = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;

    // TOTP 검증
    if !verify_totp_code(&secret_base32, email, code)? {
        return Err(Errors::TotpInvalidCode);
    }

    // 백업 코드 생성 (평문)
    let backup_codes = generate_backup_codes();
    // 해시하여 DB에 저장 (평문은 사용자에게만 반환)
    let hashed_codes = hash_backup_codes(&backup_codes);

    // DB 업데이트: totp_enabled_at 설정 + 해시된 백업 코드 저장
    repository_update_user(
        conn,
        user_id,
        UserUpdateParams {
            totp_enabled_at: Some(Some(Utc::now())),
            totp_backup_codes: Some(Some(hashed_codes)),
            ..Default::default()
        },
    )
    .await?;

    // 평문 백업 코드 반환 (사용자가 저장해야 함)
    Ok(TotpEnableResponse { backup_codes })
}
