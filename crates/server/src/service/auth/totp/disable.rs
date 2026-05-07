use super::common::verify_totp_code;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::crypto::backup_code::verify_backup_code;
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

pub async fn service_totp_disable(
    conn: &DatabaseConnection,
    user_id: Uuid,
    code: &str,
) -> ServiceResult<()> {
    let txn = conn.begin().await?;

    let user = repository_get_user_by_id(&txn, user_id).await?;

    if user.totp_enabled_at.is_none() {
        return Err(Errors::TotpNotEnabled);
    }

    let secret_base32 = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;
    let backup_codes = user.totp_backup_codes.clone().unwrap_or_default();

    if code.len() == 6 {
        if !verify_totp_code(&secret_base32, &user.email, code)? {
            return Err(Errors::TotpInvalidCode);
        }
    } else if code.len() == 8 {
        if verify_backup_code(code, &backup_codes).is_none() {
            return Err(Errors::TotpInvalidCode);
        }
    } else {
        return Err(Errors::TotpInvalidCode);
    }

    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            totp_secret: Some(None),
            totp_enabled_at: Some(None),
            totp_backup_codes: Some(None),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    info!(user_id = %user_id, "TOTP disabled");

    Ok(())
}
