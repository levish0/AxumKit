use crate::repository::auth_events::AUTH_EVENT_LOGIN_FAILED;
use crate::repository::user::repository_find_user_by_email;
use crate::service::auth::audit::{parse_ip, record_auth_event};
use crate::service::auth::device::{DeviceLoginOutcome, resolve_device_login};
use crate::service::auth::totp::TotpTempToken;
use crate::state::WorkerClient;
use dto::auth::request::LoginRequest;
use errors::errors::{Errors, ServiceResult};
use tracing::info;

use crate::utils::crypto::password::{verify_dummy_password, verify_password};
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;

/// Login outcome: session created / TOTP required / new-device verification required
pub enum LoginResult {
    /// No TOTP, trusted (recognized) device: returns the session ID
    SessionCreated {
        session_id: String,
        remember_me: bool,
    },
    /// TOTP required: returns a temporary token
    TotpRequired(String),
    /// New device: session withheld, verification email sent (OWASP ASVS 6.3.5)
    DeviceVerificationRequired,
}

/// Handles a login request.
///
/// # Responsibilities
/// - Verifies the email/password credentials.
/// - If the user has TOTP enabled, issues a temporary token and requires the TOTP step.
/// - After credentials pass: a new device requires email verification, a trusted device gets a session.
///
/// # Related
/// - `repository_find_user_by_email`
/// - `verify_password`
/// - `TotpTempToken::create`
/// - `resolve_device_login`
///
/// # Errors
/// - `Errors::InvalidCredentials` on authentication failure
/// - Returns Redis/storage errors when session/token persistence fails.
pub async fn service_login(
    db: &DatabaseConnection,
    redis: &ConnectionManager,
    worker: &WorkerClient,
    payload: LoginRequest,
    user_agent: Option<String>,
    ip_address: Option<String>,
    presented_device_token: Option<String>,
) -> ServiceResult<LoginResult> {
    let user = repository_find_user_by_email(db, payload.email.clone()).await?;
    let audit_ip = parse_ip(ip_address.as_deref());

    // Constant-time credential check (account-enumeration defense). Every path runs
    // exactly one Argon2 verification, so a missing / soft-deleted / password-less
    // (OAuth-only) account cannot be distinguished from a wrong password by timing.
    // A soft-deleted or password-less account is treated as "no usable hash" and
    // falls through to the dummy verification, then fails like any bad credential.
    match user
        .as_ref()
        .filter(|u| u.deleted_at.is_none())
        .and_then(|u| u.password.as_deref())
    {
        Some(password_hash) => {
            if verify_password(&payload.password, password_hash).is_err() {
                // Wrong password on an existing active account.
                record_auth_event(
                    db,
                    user.as_ref().map(|u| u.id),
                    AUTH_EVENT_LOGIN_FAILED,
                    audit_ip,
                    user_agent.clone(),
                    None,
                )
                .await;
                return Err(Errors::InvalidCredentials);
            }
        }
        None => {
            verify_dummy_password(&payload.password);
            // Unknown email, or a soft-deleted / password-less (OAuth-only) account.
            record_auth_event(
                db,
                user.as_ref().map(|u| u.id),
                AUTH_EVENT_LOGIN_FAILED,
                audit_ip,
                user_agent.clone(),
                None,
            )
            .await;
            return Err(Errors::InvalidCredentials);
        }
    }

    // Password matched, so an active account with a usable hash is present.
    let user = user.expect("user is present when a password hash matched");

    // Check whether TOTP is enabled
    if user.totp_enabled_at.is_some() {
        // TOTP required: create a temporary token (carries device context to chain device verification after TOTP).
        let temp_token = TotpTempToken::create(
            redis,
            user.id,
            user_agent,
            ip_address,
            payload.remember_me,
            presented_device_token,
        )
        .await?;

        info!(user_id = %user.id, "Login requires TOTP");
        return Ok(LoginResult::TotpRequired(temp_token.token));
    }

    // No TOTP: new-device check (recognized device creates a session, new device triggers an email challenge).
    let outcome = resolve_device_login(
        db,
        redis,
        worker,
        &user,
        presented_device_token,
        user_agent,
        ip_address,
        payload.remember_me,
    )
    .await?;

    match outcome {
        DeviceLoginOutcome::SessionCreated { session_token } => {
            info!(user_id = %user.id, "Login successful");
            Ok(LoginResult::SessionCreated {
                session_id: session_token,
                remember_me: payload.remember_me,
            })
        }
        DeviceLoginOutcome::VerificationRequired => {
            info!(user_id = %user.id, "Login requires new-device verification");
            Ok(LoginResult::DeviceVerificationRequired)
        }
    }
}
