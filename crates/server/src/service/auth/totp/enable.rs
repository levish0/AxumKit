use super::common::{generate_backup_codes, verify_totp_code};
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::crypto::backup_code::hash_backup_codes;
use chrono::Utc;
use dto::auth::response::TotpEnableResponse;
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

/// Enable TOTP: verify the first code, then activate and generate backup codes
pub async fn service_totp_enable(
    db: &DatabaseConnection,
    user_id: Uuid,
    code: &str,
) -> ServiceResult<TotpEnableResponse> {
    let txn = db.begin().await?;

    // Look up the user
    let user = repository_get_user_by_id(&txn, user_id).await?;

    // TOTP is already enabled
    if user.totp_enabled_at.is_some() {
        return Err(Errors::TotpAlreadyEnabled);
    }

    // No secret present (setup was never done)
    let encrypted_secret = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;
    let secret_base32 = crate::utils::crypto::totp_secret::decrypt_totp_secret(&encrypted_secret)?;

    // Verify the TOTP code
    if !verify_totp_code(&secret_base32, &user.email, code)? {
        return Err(Errors::TotpInvalidCode);
    }

    // Generate backup codes (plaintext)
    let backup_codes = generate_backup_codes();
    // Hash them for DB storage (plaintext is only returned to the user)
    let hashed_codes = hash_backup_codes(&backup_codes);

    // Update the DB: set totp_enabled_at and store the hashed backup codes
    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            totp_enabled_at: Some(Some(Utc::now())),
            totp_backup_codes: Some(Some(hashed_codes)),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    info!(user_id = %user_id, "TOTP enabled");

    // Return the plaintext backup codes (the user must save them)
    Ok(TotpEnableResponse { backup_codes })
}
