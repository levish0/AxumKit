use super::common::{generate_backup_codes, verify_totp_code};
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::crypto::backup_code::hash_backup_codes;
use dto::auth::response::TotpBackupCodesResponse;
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

pub async fn service_regenerate_backup_codes(
    conn: &DatabaseConnection,
    user_id: Uuid,
    code: &str,
) -> ServiceResult<TotpBackupCodesResponse> {
    let txn = conn.begin().await?;

    let user = repository_get_user_by_id(&txn, user_id).await?;

    if user.totp_enabled_at.is_none() {
        return Err(Errors::TotpNotEnabled);
    }

    let secret_base32 = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;

    if !verify_totp_code(&secret_base32, &user.email, code)? {
        return Err(Errors::TotpInvalidCode);
    }

    let backup_codes = generate_backup_codes();
    let hashed_codes = hash_backup_codes(&backup_codes);

    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            totp_backup_codes: Some(Some(hashed_codes)),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    Ok(TotpBackupCodesResponse { backup_codes })
}
