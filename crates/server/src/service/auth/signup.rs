use crate::bridge::worker_client;
use crate::repository::user::{repository_find_user_by_email, repository_find_user_by_handle};
use crate::service::auth::verify_email::{
    PendingEmailSignupData, find_pending_email_signup_by_email,
    find_pending_email_signup_by_handle, issue_pending_email_signup_token,
};
use crate::state::WorkerClient;
use crate::utils::crypto::password::hash_password;
use config::ServerConfig;
use dto::user::{CreateUserRequest, CreateUserResponse};
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;

/// Uniform signup response. The same message is returned whether or not the email
/// is already taken, so signup cannot be used to probe which emails are registered.
const VERIFICATION_SENT_MESSAGE: &str =
    "Verification email sent. Complete signup from the link in your inbox.";

fn verification_sent() -> CreateUserResponse {
    CreateUserResponse {
        message: VERIFICATION_SENT_MESSAGE.to_string(),
    }
}

/// Accept a signup request and defer user creation until email verification.
pub async fn service_signup(
    db: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    payload: CreateUserRequest,
) -> ServiceResult<CreateUserResponse> {
    let config = ServerConfig::get();

    // Enumeration-safe: an already-registered email returns the same response as a
    // fresh signup — no account/pending is created and no email is sent — so signup
    // does not reveal whether an email is registered (OWASP / WSTG-ATHN-03).
    // (Note: a registered email still returns faster than a fresh signup, which does
    // the pending reserve + mail send; closing that timing channel is a separate item.)
    let existing_user_by_email = repository_find_user_by_email(db, payload.email.clone()).await?;
    if existing_user_by_email.is_some() {
        return Ok(verification_sent());
    }

    // If a pending signup already exists for this email, return success without
    // overwriting. This prevents an attacker from replacing the legitimate
    // user's pending payload (password / handle) before verification.
    if find_pending_email_signup_by_email(redis_conn, &payload.email)
        .await?
        .is_some()
    {
        return Ok(verification_sent());
    }

    let existing_user_by_handle =
        repository_find_user_by_handle(db, payload.handle.clone()).await?;
    if existing_user_by_handle.is_some() {
        return Err(Errors::UserHandleAlreadyExists);
    }

    if find_pending_email_signup_by_handle(redis_conn, &payload.handle)
        .await?
        .is_some()
    {
        return Err(Errors::UserHandleAlreadyExists);
    }

    let password_hash = hash_password(&payload.password)?;
    let verification_data = PendingEmailSignupData {
        email: payload.email.clone(),
        handle: payload.handle.clone(),
        display_name: payload.display_name,
        password_hash,
    };

    let ttl_seconds = (config.auth_email_verification_token_expire_time * 60) as u64;
    let token =
        issue_pending_email_signup_token(redis_conn, &verification_data, ttl_seconds).await?;

    worker_client::send_verification_email(
        worker,
        &payload.email,
        &payload.handle,
        &token,
        config.auth_email_verification_token_expire_time as u64,
    )
    .await?;

    Ok(verification_sent())
}
