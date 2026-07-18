use super::common::verify_totp_code;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id_for_update, repository_update_user,
};
use crate::service::auth::device::{DeviceLoginOutcome, resolve_device_login};
use crate::service::auth::totp::TotpTempToken;
use crate::state::WorkerClient;
use crate::utils::crypto::backup_code::verify_backup_code;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager as RedisClient;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;

/// TOTP verification result: session created, or new-device confirmation required
pub enum TotpVerifyResult {
    /// Trusted device (or app): returns the session ID
    SessionCreated {
        session_id: String,
        remember_me: bool,
    },
    /// New device: session deferred, verification email sent (OWASP ASVS 6.3.5)
    DeviceVerificationRequired,
}

/// TOTP verification (login step 2): temp_token + code → device check, then session creation
pub async fn service_totp_verify(
    db: &DatabaseConnection,
    redis: &RedisClient,
    worker: &WorkerClient,
    temp_token: &str,
    code: &str,
) -> ServiceResult<TotpVerifyResult> {
    // Fetch and delete the temporary token (single-use)
    let token_data = TotpTempToken::get_and_delete(redis, temp_token)
        .await?
        .ok_or(Errors::TotpTempTokenInvalid)?;

    let txn = db.begin().await?;

    // Lock the user row: backup-code verification is a read-modify-write on the
    // backup_codes array, so concurrent verifications (e.g. two temp tokens reusing
    // the same backup code) must be serialized to keep each code single-use.
    let user = repository_get_user_by_id_for_update(&txn, token_data.user_id).await?;

    // TOTP must be enabled
    if user.totp_enabled_at.is_none() {
        return Err(Errors::TotpNotEnabled);
    }

    let encrypted_secret = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;
    let secret_base32 = crate::utils::crypto::totp_secret::decrypt_totp_secret(&encrypted_secret)?;
    let backup_codes = user.totp_backup_codes.clone().unwrap_or_default();

    // Distinguish TOTP vs backup code by code length
    if code.len() == 6 {
        // Verify TOTP code
        if !verify_totp_code(&secret_base32, &user.email, code)? {
            return Err(Errors::TotpInvalidCode);
        }
        // Replay guard (RFC 6238 §5.2): a valid TOTP code is single-use within its
        // validity window. Atomically claim (user_id, code); if it was already used,
        // reject — otherwise a captured code could be replayed via a fresh temp token.
        let used_key = constants::totp_used_code_key(&token_data.user_id.to_string(), code);
        let claimed = crate::utils::redis_cache::set_json_nx_with_ttl(
            redis,
            &used_key,
            &true,
            constants::TOTP_USED_CODE_TTL_SECONDS,
        )
        .await?;
        if !claimed {
            return Err(Errors::TotpInvalidCode);
        }
    } else if code.len() == 8 {
        // Verify backup code
        if backup_codes.is_empty() {
            return Err(Errors::TotpBackupCodeExhausted);
        }

        // Verify backup code via hash comparison
        if let Some(idx) = verify_backup_code(code, &backup_codes) {
            // Remove the used backup code
            let mut new_codes = backup_codes.clone();
            new_codes.remove(idx);

            repository_update_user(
                &txn,
                token_data.user_id,
                UserUpdateParams {
                    totp_backup_codes: Some(Some(new_codes)),
                    ..Default::default()
                },
            )
            .await?;
        } else {
            return Err(Errors::TotpInvalidCode);
        }
    } else {
        return Err(Errors::TotpInvalidCode);
    }

    txn.commit().await?;

    info!(user_id = %token_data.user_id, "TOTP verified");

    // Device check: decide via the device token from the initial login whether the device is
    // recognized. Recognized → create session; new device → email challenge (browser and app alike).
    let outcome = resolve_device_login(
        db,
        redis,
        worker,
        &user,
        token_data.device_token.clone(),
        token_data.user_agent.clone(),
        token_data.ip_address.clone(),
        token_data.remember_me,
    )
    .await?;

    match outcome {
        DeviceLoginOutcome::SessionCreated { session_token } => {
            Ok(TotpVerifyResult::SessionCreated {
                session_id: session_token,
                remember_me: token_data.remember_me,
            })
        }
        DeviceLoginOutcome::VerificationRequired => {
            Ok(TotpVerifyResult::DeviceVerificationRequired)
        }
    }
}
