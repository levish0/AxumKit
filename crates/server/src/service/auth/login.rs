use crate::repository::user::repository_find_user_by_email;
use crate::service::auth::session::SessionService;
use crate::service::auth::totp::TotpTempToken;
use dto::auth::request::LoginRequest;
use errors::errors::{Errors, ServiceResult};
use tracing::info;

use crate::utils::crypto::password::{verify_dummy_password, verify_password};
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;

pub enum LoginResult {
    SessionCreated {
        session_id: String,
        remember_me: bool,
    },
    TotpRequired(String),
}

pub async fn service_login(
    conn: &DatabaseConnection,
    redis: &ConnectionManager,
    payload: LoginRequest,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<LoginResult> {
    let user = repository_find_user_by_email(conn, payload.email.clone()).await?;

    // Constant-time credential check (account-enumeration defense). Every path runs
    // exactly one Argon2 verification, so a missing or password-less (OAuth-only)
    // account cannot be distinguished from a wrong password by timing.
    match user.as_ref().and_then(|u| u.password.as_deref()) {
        Some(password_hash) => {
            verify_password(&payload.password, password_hash)
                .map_err(|_| Errors::InvalidCredentials)?;
        }
        None => {
            verify_dummy_password(&payload.password);
            return Err(Errors::InvalidCredentials);
        }
    }

    // Password matched, so an account with a usable hash is present.
    let user = user.expect("user is present when a password hash matched");

    if user.totp_enabled_at.is_some() {
        let temp_token =
            TotpTempToken::create(redis, user.id, user_agent, ip_address, payload.remember_me)
                .await?;

        info!(user_id = %user.id, "Login requires TOTP");
        return Ok(LoginResult::TotpRequired(temp_token.token));
    }

    // raw_token goes out only in the cookie; the server stores its hash.
    let (raw_token, _session) =
        SessionService::create_session(redis, user.id.to_string(), user_agent, ip_address).await?;

    info!(user_id = %user.id, "Login successful");

    Ok(LoginResult::SessionCreated {
        session_id: raw_token,
        remember_me: payload.remember_me,
    })
}
