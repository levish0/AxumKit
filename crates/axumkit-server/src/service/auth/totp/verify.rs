use super::common::verify_totp_code;
use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::service::auth::session::SessionService;
use crate::service::auth::totp::TotpTempToken;
use crate::utils::crypto::backup_code::verify_backup_code;
use redis::aio::ConnectionManager as RedisClient;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use axumkit_errors::errors::{Errors, ServiceResult};

pub struct TotpVerifyResult {
    pub session_id: String,
    pub remember_me: bool,
}

pub async fn service_totp_verify(
    conn: &DatabaseConnection,
    redis: &RedisClient,
    temp_token: &str,
    code: &str,
) -> ServiceResult<TotpVerifyResult> {
    let token_data = TotpTempToken::get_and_delete(redis, temp_token)
        .await?
        .ok_or(Errors::TotpTempTokenInvalid)?;

    let txn = conn.begin().await?;

    let user = repository_get_user_by_id(&txn, token_data.user_id).await?;

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
        if backup_codes.is_empty() {
            return Err(Errors::TotpBackupCodeExhausted);
        }

        if let Some(idx) = verify_backup_code(code, &backup_codes) {
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

    let session = SessionService::create_session(
        redis,
        token_data.user_id.to_string(),
        token_data.user_agent,
        token_data.ip_address,
    )
    .await?;

    info!(user_id = %token_data.user_id, "TOTP verified");

    Ok(TotpVerifyResult {
        session_id: session.session_id,
        remember_me: token_data.remember_me,
    })
}

