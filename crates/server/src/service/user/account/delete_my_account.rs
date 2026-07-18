use crate::bridge::worker_client;
use crate::connection::r2_assets_conn::R2AssetsClient;
use crate::repository::user::repository_find_user_by_id;
use crate::service::auth::session::SessionService;
use crate::service::auth::session_types::SessionContext;
use crate::service::auth::totp::verify_totp_code;
use crate::service::blob_cleanup::delete_user_image_blob_if_unreferenced;
use crate::service::user::account::scrub::scrub_user_account;
use crate::service::user::utils::spawn_delete_user_from_index;
use crate::state::WorkerClient;
use crate::utils::crypto::backup_code::verify_backup_code;
use crate::utils::crypto::password::verify_password;
use crate::utils::crypto::token::{generate_secure_token, hash_token};
use crate::utils::crypto::totp_secret::decrypt_totp_secret;
use crate::utils::redis_cache::{get_json_and_delete, issue_token_and_store_json_with_ttl};
use config::ServerConfig;
use dto::user::DeleteMyAccountRequest;
use entity::users::Model as UserModel;
use errors::errors::Errors;
use redis::aio::ConnectionManager as RedisClient;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

/// Redis payload backing an emailed account-deletion confirmation token.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountDeletionData {
    pub user_id: String,
}

/// Result of an account-deletion request: either the account was deleted inline (after an
/// inline re-authentication factor), or a confirmation email was sent (OAuth-only accounts
/// with no inline factor).
#[derive(Debug, PartialEq, Eq)]
pub enum AccountDeletionOutcome {
    Deleted,
    ConfirmationEmailSent,
}

/// Requests deletion of the current user's account, enforcing re-authentication first.
///
/// Account deletion is irreversible, so a stolen cookie or XSS-driven request must not be able
/// to trigger it alone (OWASP ASVS 7.5.1). The re-authentication factor is chosen from the account:
/// - password set → verify `password` and delete inline;
/// - OAuth-only + TOTP enabled → verify `totp_code` (6-digit TOTP or 8-char backup) and delete inline;
/// - OAuth-only + no TOTP → no inline factor exists, so email a single-use confirmation token and
///   defer the deletion to [`service_confirm_account_deletion`].
pub async fn service_request_account_deletion(
    db: &DatabaseConnection,
    redis: &RedisClient,
    r2_assets: &R2AssetsClient,
    worker: &WorkerClient,
    session: &SessionContext,
    payload: DeleteMyAccountRequest,
) -> Result<AccountDeletionOutcome, Errors> {
    let user = repository_find_user_by_id(db, session.user_id)
        .await?
        .ok_or(Errors::UserNotFound)?;

    // Idempotent: already-deleted accounts are treated as not found.
    if user.deleted_at.is_some() {
        return Err(Errors::UserNotFound);
    }

    if let Some(password_hash) = user.password.as_deref() {
        // Password account: verify the current password inline.
        let password = payload
            .password
            .as_deref()
            .ok_or(Errors::ReauthenticationRequired)?;
        verify_password(password, password_hash)?;
        perform_account_deletion(db, redis, r2_assets, worker, user).await?;
        Ok(AccountDeletionOutcome::Deleted)
    } else if user.totp_enabled_at.is_some() {
        // OAuth-only account with 2FA: verify a TOTP or backup code inline.
        let code = payload
            .totp_code
            .as_deref()
            .ok_or(Errors::ReauthenticationRequired)?;
        verify_totp_or_backup_code(&user, code)?;
        perform_account_deletion(db, redis, r2_assets, worker, user).await?;
        Ok(AccountDeletionOutcome::Deleted)
    } else {
        // OAuth-only account with no inline factor: confirm via a single-use email token.
        let config = ServerConfig::get();
        let ttl_seconds = (config.auth_account_deletion_token_expire_time * 60) as u64;
        let data = AccountDeletionData {
            user_id: user.id.to_string(),
        };
        // Store under the hashed token id so the raw token never lives in Redis; the raw
        // token is returned and only ever sent in the confirmation email.
        let token = issue_token_and_store_json_with_ttl(
            redis,
            generate_secure_token,
            |token| constants::account_deletion_key(&hash_token(token)),
            &data,
            ttl_seconds,
        )
        .await?;

        worker_client::send_account_deletion_confirmation(
            worker,
            &user.email,
            &user.handle,
            &token,
            config.auth_account_deletion_token_expire_time as u64,
        )
        .await?;

        info!(user_id = %user.id, "Account deletion confirmation sent");
        Ok(AccountDeletionOutcome::ConfirmationEmailSent)
    }
}

/// Confirms a deferred account deletion using the single-use token from the confirmation email.
pub async fn service_confirm_account_deletion(
    db: &DatabaseConnection,
    redis: &RedisClient,
    r2_assets: &R2AssetsClient,
    worker: &WorkerClient,
    token: &str,
) -> Result<(), Errors> {
    // Single-use lookup by the hashed token id.
    let token_key = constants::account_deletion_key(&hash_token(token));
    let data: AccountDeletionData = get_json_and_delete(
        redis,
        &token_key,
        || Errors::TokenInvalidAccountDeletion,
        |_| Errors::TokenInvalidAccountDeletion,
    )
    .await?;

    let user_id =
        Uuid::parse_str(&data.user_id).map_err(|_| Errors::TokenInvalidAccountDeletion)?;

    let user = repository_find_user_by_id(db, user_id)
        .await?
        .ok_or(Errors::UserNotFound)?;
    if user.deleted_at.is_some() {
        return Err(Errors::UserNotFound);
    }

    perform_account_deletion(db, redis, r2_assets, worker, user).await
}

/// Verifies a 6-digit TOTP code or an 8-char backup code against the user's stored secret.
fn verify_totp_or_backup_code(user: &UserModel, code: &str) -> Result<(), Errors> {
    let encrypted_secret = user.totp_secret.clone().ok_or(Errors::TotpNotEnabled)?;
    let secret_base32 = decrypt_totp_secret(&encrypted_secret)?;
    let backup_codes = user.totp_backup_codes.clone().unwrap_or_default();

    let ok = if code.len() == 6 {
        verify_totp_code(&secret_base32, &user.email, code)?
    } else if code.len() == 8 {
        verify_backup_code(code, &backup_codes).is_some()
    } else {
        false
    };

    if ok {
        Ok(())
    } else {
        Err(Errors::TotpInvalidCode)
    }
}

/// Performs the irreversible account deletion for an already-authenticated user.
///
/// The user row is preserved so authored content keeps its attribution and the handle stays
/// permanently reserved. All DB mutations (PII scrub + private data deletion) happen inside one
/// transaction via [`scrub_user_account`]. Sessions, the search index entry, and R2 media are then
/// cleaned up best-effort; those failures are logged but never resurrect the account.
async fn perform_account_deletion(
    db: &DatabaseConnection,
    redis: &RedisClient,
    r2_assets: &R2AssetsClient,
    worker: &WorkerClient,
    user: UserModel,
) -> Result<(), Errors> {
    let user_id = user.id;

    // Capture storage keys to purge after the row no longer references them.
    let profile_image = user.profile_image.clone();
    let banner_image = user.banner_image.clone();

    let txn = db.begin().await?;
    scrub_user_account(&txn, user_id).await?;
    txn.commit().await?;

    // Content-addressed images may be shared across users, so delete each only when no
    // user references it anymore (the row was already scrubbed above).
    if let Some(storage_key) = profile_image {
        delete_user_image_blob_if_unreferenced(db, r2_assets, &storage_key).await;
    }
    if let Some(storage_key) = banner_image {
        delete_user_image_blob_if_unreferenced(db, r2_assets, &storage_key).await;
    }

    if let Err(e) = SessionService::delete_all_user_sessions(redis, &user_id.to_string()).await {
        warn!(user_id = %user_id, error = ?e, "Failed to delete sessions after account deletion");
    }
    spawn_delete_user_from_index(worker, user_id);
    Ok(())
}
