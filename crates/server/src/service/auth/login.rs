use crate::repository::auth_events::AUTH_EVENT_LOGIN_FAILED;
use crate::repository::user::repository_find_user_by_email;
use crate::service::auth::audit::{parse_ip, record_auth_event};
use crate::service::auth::device::{DeviceCheck, DeviceLoginOutcome, resolve_device_login};
use crate::service::auth::totp::TotpTempToken;
use crate::state::WorkerClient;
use dto::auth::request::LoginRequest;
use errors::errors::{Errors, ServiceResult};
use tracing::info;

use crate::utils::crypto::password::{verify_dummy_password, verify_password};
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;

/// Login outcome: session created / TOTP required / new-device verification required.
pub enum LoginResult {
    /// No TOTP, trusted device (or app): the session token is returned.
    SessionCreated {
        session_id: String,
        remember_me: bool,
    },
    /// TOTP required: a temporary token is returned.
    TotpRequired(String),
    /// New device: the session is withheld and a verification email was sent (OWASP ASVS 6.3.5).
    DeviceVerificationRequired,
}

pub async fn service_login(
    db: &DatabaseConnection,
    redis: &ConnectionManager,
    worker: &WorkerClient,
    payload: LoginRequest,
    user_agent: Option<String>,
    ip_address: Option<String>,
    device_check: DeviceCheck,
) -> ServiceResult<LoginResult> {
    let user = repository_find_user_by_email(db, payload.email.clone()).await?;
    let audit_ip = parse_ip(ip_address.as_deref());

    // Device context to carry through a TOTP step (browser flow applies verification; app skips).
    let (device_token, apply_device_check) = match &device_check {
        DeviceCheck::Browser(token) => (token.clone(), true),
        DeviceCheck::Skip => (None, false),
    };

    // Constant-time credential check (account-enumeration defense). Every path runs
    // exactly one Argon2 verification, so a missing or password-less (OAuth-only)
    // account cannot be distinguished from a wrong password by timing.
    match user.as_ref().and_then(|u| u.password.as_deref()) {
        Some(password_hash) => {
            if verify_password(&payload.password, password_hash).is_err() {
                // Wrong password on an existing account.
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
            // Unknown email, or a password-less (OAuth-only) account.
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

    // Password matched, so an account with a usable hash is present.
    let user = user.expect("user is present when a password hash matched");

    if user.totp_enabled_at.is_some() {
        // TOTP required: create the temp token, carrying the device context so new-device
        // verification can run after the TOTP step.
        let temp_token = TotpTempToken::create(
            redis,
            user.id,
            user_agent,
            ip_address,
            payload.remember_me,
            device_token,
            apply_device_check,
        )
        .await?;

        info!(user_id = %user.id, "Login requires TOTP");
        return Ok(LoginResult::TotpRequired(temp_token.token));
    }

    // No TOTP: new-device verification (trusted device → session; new device → email challenge).
    let outcome = resolve_device_login(
        db,
        redis,
        worker,
        &user,
        device_check,
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
