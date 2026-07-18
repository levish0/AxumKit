use super::common::{generate_backup_codes, verify_totp_code};
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::crypto::backup_code::hash_backup_codes;
use dto::auth::response::TotpBackupCodesResponse;
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

/// Regenerate backup codes: verify the current TOTP code, then generate new backup codes
pub async fn service_regenerate_backup_codes(
    db: &DatabaseConnection,
    user_id: Uuid,
    code: &str,
) -> ServiceResult<TotpBackupCodesResponse> {
    let txn = db.begin().await?;

    // Look up the user
    let user = repository_get_user_by_id(&txn, user_id).await?;

    // TOTP must be enabled
    if user.totp_enabled_at.is_none() {
        return Err(Errors::TotpNotEnabled);
    }

    let encrypted_secret = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;
    let secret_base32 = crate::utils::crypto::totp_secret::decrypt_totp_secret(&encrypted_secret)?;

    // Verify the TOTP code (regenerating backup codes requires a TOTP code, never a backup code)
    if !verify_totp_code(&secret_base32, &user.email, code)? {
        return Err(Errors::TotpInvalidCode);
    }

    // Generate new backup codes (plaintext)
    let backup_codes = generate_backup_codes();
    // Hash them for DB storage
    let hashed_codes = hash_backup_codes(&backup_codes);

    // Update the DB
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

    // Return the plaintext backup codes (the user must save them)
    Ok(TotpBackupCodesResponse { backup_codes })
}
