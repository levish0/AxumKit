use super::common::verify_totp_code;
use crate::bridge::worker_client;
use crate::repository::auth_events::AUTH_EVENT_TOTP_DISABLED;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::service::auth::audit::record_auth_event;
use crate::state::WorkerClient;
use crate::utils::crypto::backup_code::verify_backup_code;
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

pub async fn service_totp_disable(
    conn: &DatabaseConnection,
    worker: &WorkerClient,
    user_id: Uuid,
    code: &str,
) -> ServiceResult<()> {
    let txn = conn.begin().await?;

    let user = repository_get_user_by_id(&txn, user_id).await?;
    let email = user.email.clone();
    let handle = user.handle.clone();

    if user.totp_enabled_at.is_none() {
        return Err(Errors::TotpNotEnabled);
    }

    let encrypted_secret = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;
    let secret_base32 = crate::utils::crypto::totp_secret::decrypt_totp_secret(&encrypted_secret)?;
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

    // Durable audit + owner notification of the 2FA change (OWASP ASVS 6.3.7).
    record_auth_event(conn, Some(user_id), AUTH_EVENT_TOTP_DISABLED, None, None, None).await;
    if let Err(e) = worker_client::send_security_alert(
        worker,
        &email,
        &handle,
        "Two-factor authentication (TOTP) was disabled",
    )
    .await
    {
        tracing::warn!(user_id = %user_id, error = ?e, "Failed to queue TOTP-disable alert email");
    }

    Ok(())
}
