use crate::repository::user::{
    repository_create_user_with_password_hash, repository_find_user_by_email,
    repository_find_user_by_handle,
};
use crate::utils::crypto::token::generate_secure_token;
use crate::utils::redis_cache::{delete_key, get_json, get_ttl_seconds};
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

    let email_key = constants::email_signup_email_key(&signup_data.email);
    let handle_key = constants::email_signup_handle_key(&signup_data.handle);
    let token_key = constants::email_verification_key(&token);

    let token_json = serde_json::to_string(&token).map_err(|e| {
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
        .arg(&token_json)
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
    find_pending_email_signup_by_index(redis_conn, &constants::email_signup_email_key(email)).await
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
        &constants::email_signup_email_key(&signup_data.email),
    )
    .await?;
    delete_key(
        redis_conn,
        &constants::email_signup_handle_key(&signup_data.handle),
    )
    .await?;

    Ok(())
}

/// Get the remaining TTL (in minutes, rounded up) of a pending signup token.
pub async fn get_pending_signup_remaining_minutes(
    redis_conn: &ConnectionManager,
    token: &str,
) -> ServiceResult<u64> {
    let key = constants::email_verification_key(token);
    match get_ttl_seconds(redis_conn, &key).await? {
        Some(secs) => Ok(secs.div_ceil(60)),
        None => Ok(0),
    }
}

async fn find_pending_email_signup_by_index(
    redis_conn: &ConnectionManager,
    index_key: &str,
) -> ServiceResult<Option<(String, PendingEmailSignupData)>> {
    let Some(token) = get_json::<String>(redis_conn, index_key).await? else {
        return Ok(None);
    };

    let verification_key = constants::email_verification_key(&token);
    let Some(signup_data) =
        get_json::<PendingEmailSignupData>(redis_conn, &verification_key).await?
    else {
        return Ok(None);
    };

    Ok(Some((token, signup_data)))
}

/// Verify a pending signup email token and create the user account.
pub async fn service_verify_email(
    db: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    token: &str,
) -> ServiceResult<Uuid> {
    let token_key = constants::email_verification_key(token);

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
