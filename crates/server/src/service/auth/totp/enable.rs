use super::common::{generate_backup_codes, verify_totp_code};
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::crypto::backup_code::hash_backup_codes;
use dto::auth::response::TotpEnableResponse;
use errors::errors::{Errors, ServiceResult};
use chrono::Utc;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

pub async fn service_totp_enable(
    conn: &DatabaseConnection,
    user_id: Uuid,
    code: &str,
) -> ServiceResult<TotpEnableResponse> {
    let txn = conn.begin().await?;

    let user = repository_get_user_by_id(&txn, user_id).await?;

    if user.totp_enabled_at.is_some() {
        return Err(Errors::TotpAlreadyEnabled);
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
            totp_enabled_at: Some(Some(Utc::now())),
            totp_backup_codes: Some(Some(hashed_codes)),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    info!(user_id = %user_id, "TOTP enabled");

    Ok(TotpEnableResponse { backup_codes })
}
