use super::common::ISSUER;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use axumkit_dto::auth::response::TotpSetupResponse;
use axumkit_errors::errors::{Errors, ServiceResult};
use rand::RngExt;
use sea_orm::{DatabaseConnection, TransactionTrait};
use totp_rs::{Algorithm, Secret, TOTP};
use tracing::info;
use uuid::Uuid;

pub async fn service_totp_setup(
    conn: &DatabaseConnection,
    user_id: Uuid,
) -> ServiceResult<TotpSetupResponse> {
    let txn = conn.begin().await?;

    let user = repository_get_user_by_id(&txn, user_id).await?;

    if user.totp_enabled_at.is_some() {
        return Err(Errors::TotpAlreadyEnabled);
    }

    let (secret_bytes, secret_base32) = {
        let mut rng = rand::rng();
        let bytes: [u8; 20] = rng.random();
        let secret = Secret::Raw(bytes.to_vec());
        (bytes, secret.to_encoded().to_string())
    };

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

    let qr_code_uri = totp.get_url();
    let qr_code_png_base64 = totp
        .get_qr_base64()
        .map_err(|_| Errors::TotpQrGenerationFailed)?;

    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            totp_secret: Some(Some(secret_base32)),
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
