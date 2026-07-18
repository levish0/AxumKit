use crate::bridge::worker_client;
use crate::repository::auth_events::AUTH_EVENT_EMAIL_CHANGED;
use crate::repository::user::{
    UserUpdateParams, repository_find_user_by_email, repository_get_user_by_id,
    repository_update_user,
};
use crate::service::auth::audit::record_auth_event;
use crate::service::auth::change_email::EmailChangeData;
use crate::service::auth::session::SessionService;
use crate::state::WorkerClient;
use crate::utils::crypto::token::hash_token;
use crate::utils::redis_cache::get_json_and_delete;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

/// Confirms an email change.
pub async fn service_confirm_email_change(
    db: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    token: &str,
) -> ServiceResult<()> {
    // 1. Validate the token in Redis (single-use via get_del). Look up by the hashed token id.
    let token_key = constants::email_change_key(&hash_token(token));
    let change_data: EmailChangeData = get_json_and_delete(
        redis_conn,
        &token_key,
        || Errors::TokenInvalidEmailChange,
        |_| Errors::TokenInvalidEmailChange,
    )
    .await?;

    // 2. Parse user_id
    let user_id =
        Uuid::parse_str(&change_data.user_id).map_err(|_| Errors::TokenInvalidEmailChange)?;

    let txn = db.begin().await?;

    // Capture the outgoing (old) address before the update so we can alert it below — the owner of
    // the previous address must learn their account was moved off it, even though the confirmation
    // link only reached the new address.
    let previous = repository_get_user_by_id(&txn, user_id).await?;
    let old_email = previous.email.clone();
    let handle = previous.handle.clone();

    // 3. Duplicate-email check (another user may have taken the email since the token was issued)
    if let Some(existing) =
        repository_find_user_by_email(&txn, change_data.new_email.clone()).await?
        && existing.id != user_id
    {
        return Err(Errors::UserEmailAlreadyExists);
    }

    // 4. Update the email
    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            email: Some(change_data.new_email.clone()),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    // 5. Changing the login identifier invalidates every active session: any session opened under
    // the old identity (including a hijacker's) is force-logged-out, so the change can't be used to
    // silently retain access. The owner re-authenticates with the new address.
    let deleted_count =
        SessionService::delete_all_user_sessions(redis_conn, &user_id.to_string()).await?;

    info!(user_id = %user_id, invalidated_sessions = deleted_count, "Email changed");

    // 6. Durable audit + alert BOTH the old and new addresses (OWASP ASVS 6.3.7). Best-effort —
    // a notification failure must not fail a completed change.
    record_auth_event(
        db,
        Some(user_id),
        AUTH_EVENT_EMAIL_CHANGED,
        None,
        None,
        None,
    )
    .await;
    if let Err(e) = worker_client::send_security_alert(
        worker,
        &old_email,
        &handle,
        "Your account email address was changed",
    )
    .await
    {
        tracing::warn!(user_id = %user_id, error = ?e, "Failed to queue email-changed alert to old address");
    }
    if let Err(e) = worker_client::send_security_alert(
        worker,
        &change_data.new_email,
        &handle,
        "Your account email address was changed",
    )
    .await
    {
        tracing::warn!(user_id = %user_id, error = ?e, "Failed to queue email-changed alert to new address");
    }

    Ok(())
}
