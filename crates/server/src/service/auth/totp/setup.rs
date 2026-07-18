use super::common::ISSUER;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use dto::auth::response::TotpSetupResponse;
use errors::errors::{Errors, ServiceResult};
use rand::RngExt;
use sea_orm::{DatabaseConnection, TransactionTrait};
use totp_rs::{Algorithm, Secret, TOTP};
use tracing::info;
use uuid::Uuid;

/// Start TOTP setup: generate the secret, store it in the DB (not yet enabled), return the QR
pub async fn service_totp_setup(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> ServiceResult<TotpSetupResponse> {
    let txn = db.begin().await?;

    // Look up the user
    let user = repository_get_user_by_id(&txn, user_id).await?;

    // TOTP is already enabled
    if user.totp_enabled_at.is_some() {
        return Err(Errors::TotpAlreadyEnabled);
    }

    // Generate the secret (20 bytes = 160 bits, recommended by RFC 4226)
    let (secret_bytes, secret_base32) = {
        let mut rng = rand::rng();
        let bytes: [u8; 20] = rng.random();
        let secret = Secret::Raw(bytes.to_vec());
        (bytes, secret.to_encoded().to_string())
    };

    // Create the TOTP instance
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,  // digits
        1,  // skew
        30, // step
        secret_bytes.to_vec(),
        Some(ISSUER.to_string()),
        user.email,
    )
    .map_err(|_| Errors::TotpSecretGenerationFailed)?;

    // Generate the QR code (PNG base64)
    let qr_code_uri = totp.get_url();
    let qr_code_png_base64 = totp
        .get_qr_base64()
        .map_err(|_| Errors::TotpQrGenerationFailed)?;

    // Store the secret in the DB (encrypted; totp_enabled_at stays NULL for now)
    let encrypted_secret = crate::utils::crypto::totp_secret::encrypt_totp_secret(&secret_base32)?;
    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            totp_secret: Some(Some(encrypted_secret)),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    info!(user_id = %user_id, "TOTP setup initiated");

    Ok(TotpSetupResponse {
        qr_code_base64: qr_code_png_base64,
        qr_code_uri,
    })
}
