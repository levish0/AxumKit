use crate::bridge::worker_client;
use crate::repository::auth_events::AUTH_EVENT_PASSWORD_CHANGED;
use crate::repository::user::UserUpdateParams;
use crate::repository::user::repository_get_user_by_id;
use crate::repository::user::repository_update_user;
use crate::service::auth::audit::record_auth_event;
use crate::service::auth::session::SessionService;
use crate::state::WorkerClient;
use crate::utils::crypto::password::{hash_password, verify_password};
use dto::auth::request::ChangePasswordRequest;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

/// Changes the user's password.
///
/// # Arguments
/// * `db` - Database connection
/// * `redis_conn` - Redis connection
/// * `user_id` - User ID
/// * `session_id` - Current session ID (the session to keep)
/// * `payload` - Password change request
pub async fn service_change_password(
    db: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    user_id: Uuid,
    session_id: &str,
    payload: ChangePasswordRequest,
) -> ServiceResult<()> {
    let txn = db.begin().await?;

    // 1. Look up the user
    let user = repository_get_user_by_id(&txn, user_id).await?;
    let email = user.email.clone();
    let handle = user.handle.clone();

    // 2. Ensure a password is set (excludes OAuth-only users)
    let password_hash = user.password.ok_or(Errors::UserPasswordNotSet)?;

    // 3. Verify the current password
    verify_password(&payload.current_password, &password_hash)?;

    // 4. Check that the new password differs from the current one
    if payload.current_password == payload.new_password {
        return Err(Errors::BadRequestError(
            "New password must be different from current password.".to_string(),
        ));
    }

    // 5. Hash the new password
    let new_password_hash = hash_password(&payload.new_password)?;

    // 6. Update the password
    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            password: Some(Some(new_password_hash)),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    // 7. Invalidate all sessions except the current one
    let deleted_count =
        SessionService::delete_other_sessions(redis_conn, &user_id.to_string(), session_id).await?;

    info!(user_id = %user_id, invalidated_sessions = deleted_count, "Password changed");

    // Durable audit + owner notification of the credential change (OWASP ASVS 6.3.7).
    record_auth_event(
        db,
        Some(user_id),
        AUTH_EVENT_PASSWORD_CHANGED,
        None,
        None,
        None,
    )
    .await;
    if let Err(e) =
        worker_client::send_security_alert(worker, &email, &handle, "Your password was changed")
            .await
    {
        tracing::warn!(user_id = %user_id, error = ?e, "Failed to queue password-change alert email");
    }

    Ok(())
}
