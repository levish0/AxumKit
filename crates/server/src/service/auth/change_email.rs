use crate::bridge::worker_client;
use crate::repository::auth_events::AUTH_EVENT_EMAIL_CHANGE_REQUESTED;
use crate::repository::user::{repository_find_user_by_email, repository_get_user_by_id};
use crate::service::auth::audit::record_auth_event;
use crate::state::WorkerClient;
use crate::utils::crypto::password::verify_password;
use crate::utils::crypto::token::{generate_secure_token, hash_token};
use crate::utils::email::normalize_email;
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use config::ServerConfig;
use dto::auth::request::ChangeEmailRequest;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
/// Data structure for email change data.
pub struct EmailChangeData {
    pub user_id: String,
    pub new_email: String,
}

/// Requests an email change. A verification email is sent to the new address.
pub async fn service_change_email(
    db: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    user_id: Uuid,
    payload: ChangeEmailRequest,
) -> ServiceResult<()> {
    let config = ServerConfig::get();

    // 1. Look up the user
    let user = repository_get_user_by_id(db, user_id).await?;

    // 2. Verify the password (OAuth-only users cannot change it)
    let password_hash = user.password.ok_or(Errors::UserPasswordNotSet)?;
    verify_password(&payload.password, &password_hash)?;

    // Canonicalize once so the comparison and storage match the repository, which
    // normalizes email on lookup/write.
    let new_email = normalize_email(&payload.new_email);

    // 3. Check that the new email differs from the current one (case/whitespace insensitive)
    if user.email == new_email {
        return Err(Errors::BadRequestError(
            "New email must be different from current email.".to_string(),
        ));
    }

    // 4. Check whether the new email is already in use
    if repository_find_user_by_email(db, new_email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::UserEmailAlreadyExists);
    }

    // 5. Generate the token

    let change_data = EmailChangeData {
        user_id: user.id.to_string(),
        new_email,
    };

    // 6. Store the token in Redis (convert minutes to seconds)
    let ttl_seconds = (config.auth_email_change_token_expire_time * 60) as u64;
    // Store under the hashed token id so the raw token never lives in Redis;
    // the raw token is returned and only ever sent in the email link.
    let token = issue_token_and_store_json_with_ttl(
        redis_conn,
        generate_secure_token,
        |token| constants::email_change_key(&hash_token(token)),
        &change_data,
        ttl_seconds,
    )
    .await?;

    // 7. Ask the Worker service to send the email (to the normalized new address)
    worker_client::send_email_change_verification(
        worker,
        &change_data.new_email,
        &user.handle,
        &token,
        config.auth_email_change_token_expire_time as u64,
    )
    .await?;

    info!(user_id = %user_id, "Email change verification sent");

    // 8. Durable audit + alert the CURRENT address (OWASP: notify the account owner of a pending
    // credential-identifier change). The confirmation link only reaches the *new* address, so
    // without this the legitimate owner would never learn a change was initiated on their account.
    // Best-effort — a notification failure must not fail an accepted change request.
    record_auth_event(
        db,
        Some(user_id),
        AUTH_EVENT_EMAIL_CHANGE_REQUESTED,
        None,
        None,
        None,
    )
    .await;
    if let Err(e) = worker_client::send_security_alert(
        worker,
        &user.email,
        &user.handle,
        "An email change was requested for your account",
    )
    .await
    {
        tracing::warn!(user_id = %user_id, error = ?e, "Failed to queue email-change alert to current address");
    }

    Ok(())
}
