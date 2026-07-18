use crate::repository::user::{
    repository_create_user_with_password_hash, repository_find_user_by_email,
    repository_find_user_by_handle,
};
use crate::utils::crypto::token::{generate_secure_token, hash_token};
use crate::utils::email::normalize_email;
use crate::utils::redis_cache::{delete_key, get_json, get_ttl_seconds, set_json_with_ttl};
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use tracing::info;
use uuid::Uuid;

static RESERVE_PENDING_SIGNUP_SCRIPT: LazyLock<redis::Script> =
    LazyLock::new(|| redis::Script::new(include_str!("lua/reserve_pending_signup.lua")));

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Pending email/password signup payload stored in Redis until verification.
pub struct PendingEmailSignupData {
    pub email: String,
    pub handle: String,
    pub display_name: String,
    pub password_hash: String,
}

/// Issue a new pending email signup token, reserving email index, handle index,
/// and token payload **atomically** via a Lua script.
/// Returns `Err` if either index key already exists.
pub async fn issue_pending_email_signup_token(
    redis_conn: &ConnectionManager,
    signup_data: &PendingEmailSignupData,
    ttl_seconds: u64,
) -> ServiceResult<String> {
    let token = generate_secure_token();
    // Everything stored in Redis keys off the hashed token id; the raw token only
    // ever leaves in the email link. The email/handle index values hold the id too
    // (not the raw token), so a store leak yields no usable verification link.
    let token_id = hash_token(&token);

    // Email index is normalized (case-insensitive) so case variants dedup to one
    // pending record; handle stays case-sensitive (handles are case-sensitive).
    let email_key = constants::email_signup_email_key(&normalize_email(&signup_data.email));
    let handle_key = constants::email_signup_handle_key(&signup_data.handle);
    let token_key = constants::email_verification_key(&token_id);

    let token_id_json = serde_json::to_string(&token_id).map_err(|e| {
        Errors::SysInternalError(format!("JSON serialization failed for token index: {}", e))
    })?;

    let payload_json = serde_json::to_string(signup_data).map_err(|e| {
        Errors::SysInternalError(format!(
            "JSON serialization failed for signup payload: {}",
            e
        ))
    })?;

    let mut conn = redis_conn.clone();
    let result: i64 = RESERVE_PENDING_SIGNUP_SCRIPT
        .key(&email_key)
        .key(&handle_key)
        .key(&token_key)
        .arg(&token_id_json)
        .arg(&payload_json)
        .arg(ttl_seconds)
        .invoke_async(&mut conn)
        .await
        .map_err(|e| {
            Errors::SysInternalError(format!("Redis reserve_pending_signup script failed: {}", e))
        })?;

    match result {
        1 => Ok(token),
        -1 => Err(Errors::UserEmailAlreadyExists),
        -2 => Err(Errors::UserHandleAlreadyExists),
        other => Err(Errors::SysInternalError(format!(
            "Unexpected reserve_pending_signup result: {}",
            other
        ))),
    }
}

pub async fn find_pending_email_signup_by_email(
    redis_conn: &ConnectionManager,
    email: &str,
) -> ServiceResult<Option<(String, PendingEmailSignupData)>> {
    find_pending_email_signup_by_index(
        redis_conn,
        &constants::email_signup_email_key(&normalize_email(email)),
    )
    .await
}

pub async fn find_pending_email_signup_by_handle(
    redis_conn: &ConnectionManager,
    handle: &str,
) -> ServiceResult<Option<(String, PendingEmailSignupData)>> {
    find_pending_email_signup_by_index(redis_conn, &constants::email_signup_handle_key(handle))
        .await
}

pub async fn delete_pending_email_signup_indices(
    redis_conn: &ConnectionManager,
    signup_data: &PendingEmailSignupData,
) -> ServiceResult<()> {
    delete_key(
        redis_conn,
        &constants::email_signup_email_key(&normalize_email(&signup_data.email)),
    )
    .await?;
    delete_key(
        redis_conn,
        &constants::email_signup_handle_key(&signup_data.handle),
    )
    .await?;

    Ok(())
}

/// Re-issue a fresh verification token for an existing pending signup, preserving
/// the remaining validity window and invalidating the previous token.
///
/// Used by resend: since only the hashed token id is stored, the original raw
/// token cannot be recovered to re-send, so a new one is minted. The payload and
/// email/handle indices are repointed at the new id and the old payload key is
/// deleted, so the previous link stops working. Returns `(new_raw_token,
/// remaining_minutes)`, or `None` if the pending signup has already expired.
pub async fn reissue_pending_email_signup_token(
    redis_conn: &ConnectionManager,
    old_token_id: &str,
    signup_data: &PendingEmailSignupData,
) -> ServiceResult<Option<(String, u64)>> {
    let old_key = constants::email_verification_key(old_token_id);
    let Some(remaining_secs) = get_ttl_seconds(redis_conn, &old_key).await? else {
        return Ok(None);
    };
    if remaining_secs == 0 {
        return Ok(None);
    }

    let new_token = generate_secure_token();
    let new_token_id = hash_token(&new_token);
    let new_key = constants::email_verification_key(&new_token_id);
    // Email index is normalized (case-insensitive) so case variants dedup to one
    // pending record; handle stays case-sensitive (handles are case-sensitive).
    let email_key = constants::email_signup_email_key(&normalize_email(&signup_data.email));
    let handle_key = constants::email_signup_handle_key(&signup_data.handle);

    // Repoint payload + indices at the new id within the remaining window, then drop
    // the old payload so the previous link no longer resolves.
    set_json_with_ttl(redis_conn, &new_key, signup_data, remaining_secs).await?;
    set_json_with_ttl(redis_conn, &email_key, &new_token_id, remaining_secs).await?;
    set_json_with_ttl(redis_conn, &handle_key, &new_token_id, remaining_secs).await?;
    delete_key(redis_conn, &old_key).await.ok();

    Ok(Some((new_token, remaining_secs.div_ceil(60))))
}

async fn find_pending_email_signup_by_index(
    redis_conn: &ConnectionManager,
    index_key: &str,
) -> ServiceResult<Option<(String, PendingEmailSignupData)>> {
    // The index value is the hashed token id (not the raw token).
    let Some(token_id) = get_json::<String>(redis_conn, index_key).await? else {
        return Ok(None);
    };

    let verification_key = constants::email_verification_key(&token_id);
    let Some(signup_data) =
        get_json::<PendingEmailSignupData>(redis_conn, &verification_key).await?
    else {
        // Token payload is gone (consumed by verify or expired).
        // Don't eagerly delete the index — it will expire via its own TTL,
        // and deleting here would open a race window that lets a concurrent
        // create_user re-reserve the same email/handle.
        return Ok(None);
    };

    Ok(Some((token_id, signup_data)))
}

/// Verify a pending signup email token and create the user account.
///
/// The data is read **without** deleting first. Redis keys are only cleaned up
/// after the DB commit succeeds, so a transient Postgres failure does not lose
/// the pending signup.
pub async fn service_verify_email(
    db: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    token: &str,
) -> ServiceResult<Uuid> {
    let token_key = constants::email_verification_key(&hash_token(token));

    let signup_data: PendingEmailSignupData = get_json(redis_conn, &token_key)
        .await?
        .ok_or(Errors::TokenInvalidVerification)?;

    let user_id = complete_pending_email_signup(db, signup_data.clone()).await?;

    // DB commit succeeded — now clean up Redis (best-effort).
    delete_key(redis_conn, &token_key).await.ok();
    delete_pending_email_signup_indices(redis_conn, &signup_data)
        .await
        .ok();

    Ok(user_id)
}

async fn complete_pending_email_signup(
    db: &DatabaseConnection,
    signup_data: PendingEmailSignupData,
) -> ServiceResult<Uuid> {
    let txn = db.begin().await?;

    if repository_find_user_by_email(&txn, signup_data.email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::UserEmailAlreadyExists);
    }

    if repository_find_user_by_handle(&txn, signup_data.handle.clone())
        .await?
        .is_some()
    {
        return Err(Errors::UserHandleAlreadyExists);
    }

    let user = repository_create_user_with_password_hash(
        &txn,
        signup_data.email,
        signup_data.handle,
        signup_data.display_name,
        signup_data.password_hash,
    )
    .await?;

    txn.commit().await?;

    info!(user_id = %user.id, handle = %user.handle, "Pending signup completed");

    Ok(user.id)
}
