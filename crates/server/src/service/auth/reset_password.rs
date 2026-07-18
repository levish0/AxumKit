use crate::bridge::worker_client;
use crate::repository::auth_events::AUTH_EVENT_PASSWORD_RESET;
use crate::repository::user::{
    UserUpdateParams, repository_find_user_by_id, repository_update_user,
};
use crate::service::auth::audit::record_auth_event;
use crate::service::auth::forgot_password::PasswordResetData;
use crate::service::auth::session::SessionService;
use crate::state::WorkerClient;
use crate::utils::crypto::password::hash_password;
use crate::utils::crypto::token::hash_token;
use crate::utils::redis_cache::get_json_and_delete;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;
use tracing::info;
use uuid::Uuid;

/// Resets the user's password.
///
/// # Arguments
/// * `conn` - Database connection
/// * `redis_conn` - Redis connection
/// * `token` - Password reset token
/// * `new_password` - New password
pub async fn service_reset_password(
    db: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    token: &str,
    new_password: &str,
) -> ServiceResult<()> {
    // 1. Validate the token in Redis (single-use via get_del). Look up by the hashed token id.
    let token_key = constants::password_reset_key(&hash_token(token));
    let reset_data: PasswordResetData = get_json_and_delete(
        redis_conn,
        &token_key,
        || Errors::TokenInvalidReset,
        |_| Errors::TokenInvalidReset,
    )
    .await?;

    // 2. Parse user_id
    let user_id = Uuid::parse_str(&reset_data.user_id).map_err(|_| Errors::TokenInvalidReset)?;

    // Reject soft-deleted accounts (defense-in-depth; a scrubbed user has no password to reset).
    let user = repository_find_user_by_id(db, user_id)
        .await?
        .ok_or(Errors::TokenInvalidReset)?;
    if user.deleted_at.is_some() {
        return Err(Errors::TokenInvalidReset);
    }

    // 3. Hash the new password
    let password_hash = hash_password(new_password)?;

    // 4. Update the password
    repository_update_user(
        db,
        user_id,
        UserUpdateParams {
            password: Some(Some(password_hash)),
            ..Default::default()
        },
    )
    .await?;

    // 5. Invalidate all sessions for the user
    let deleted_count =
        SessionService::delete_all_user_sessions(redis_conn, &user_id.to_string()).await?;

    info!(user_id = %user_id, invalidated_sessions = deleted_count, "Password reset completed");

    // 6. Durable audit + owner notification of the credential change (OWASP ASVS 6.3.7),
    // mirroring the password-change flow: a successful reset (a full account-takeover primitive)
    // must leave a record and alert the account owner. Best-effort — a notification failure must
    // not fail a completed reset.
    record_auth_event(
        db,
        Some(user_id),
        AUTH_EVENT_PASSWORD_RESET,
        None,
        None,
        None,
    )
    .await;
    if let Err(e) = worker_client::send_security_alert(
        worker,
        &user.email,
        &user.handle,
        "Your password was reset",
    )
    .await
    {
        tracing::warn!(user_id = %user_id, error = ?e, "Failed to queue password-reset alert email");
    }

    Ok(())
}
