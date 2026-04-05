use super::change_email::auth_change_email;
use super::change_password::auth_change_password;
use super::complete_signup::auth_complete_signup;
use super::confirm_email_change::auth_confirm_email_change;
use super::forgot_password::auth_forgot_password;
use super::login::auth_login;
use super::logout::auth_logout;
use super::resend_verification_email::auth_resend_verification_email;
use super::reset_password::auth_reset_password;
use super::totp::disable::totp_disable;
use super::totp::enable::totp_enable;
use super::totp::regenerate_backup_codes::totp_regenerate_backup_codes;
use super::totp::setup::totp_setup;
use super::totp::status::totp_status;
use super::totp::verify::totp_verify;
use super::verify_email::auth_verify_email;
use crate::api::v0::routes::auth::oauth::github::github_authorize::auth_github_authorize;
use crate::api::v0::routes::auth::oauth::github::github_link::auth_github_link;
use crate::api::v0::routes::auth::oauth::github::github_login::auth_github_login;
use crate::api::v0::routes::auth::oauth::google::google_authorize::auth_google_authorize;
use crate::api::v0::routes::auth::oauth::google::google_link::auth_google_link;
use crate::api::v0::routes::auth::oauth::google::google_login::auth_google_login;
use crate::api::v0::routes::auth::oauth::list_oauth_connections::list_oauth_connections;
use crate::api::v0::routes::auth::oauth::unlink_oauth_connection::unlink_oauth_connection;
use crate::state::AppState;
use axum::{Router, routing::get, routing::post};

pub fn auth_routes(_state: AppState) -> Router<AppState> {
    Router::new()
        // Protected routes (authentication via extractors)
        .route("/auth/logout", post(auth_logout))
        .route("/auth/oauth/connections", get(list_oauth_connections))
        .route(
            "/auth/oauth/connections/unlink",
            post(unlink_oauth_connection),
        )
        .route(
            "/auth/resend-verification-email",
            post(auth_resend_verification_email),
        )
        // TOTP protected routes (require session)
        .route("/auth/totp/status", get(totp_status))
        .route("/auth/totp/disable", post(totp_disable))
        // OAuth authorize routes (URL generation)
        .route("/auth/oauth/google/authorize", get(auth_google_authorize))
        .route("/auth/oauth/github/authorize", get(auth_github_authorize))
        // OAuth login routes (code exchange)
        .route("/auth/oauth/google/login", post(auth_google_login))
        .route("/auth/oauth/github/login", post(auth_github_login))
        // OAuth complete signup (pending token + handle)
        .route("/auth/complete-signup", post(auth_complete_signup))
        // OAuth link routes (link existing account)
        .route("/auth/oauth/google/link", post(auth_google_link))
        .route("/auth/oauth/github/link", post(auth_github_link))
        // Email/password login route
        .route("/auth/login", post(auth_login))
        // Email verification route (public)
        .route("/auth/verify-email", post(auth_verify_email))
        // TOTP setup/enable routes (require session)
        .route("/auth/totp/setup", post(totp_setup))
        .route("/auth/totp/enable", post(totp_enable))
        // TOTP verify route (public, for 2FA login)
        .route("/auth/totp/verify", post(totp_verify))
        // TOTP backup codes regeneration (require session)
        .route(
            "/auth/totp/backup-codes/regenerate",
            post(totp_regenerate_backup_codes),
        )
        // Password reset routes
        .route("/auth/forgot-password", post(auth_forgot_password))
        .route("/auth/reset-password", post(auth_reset_password))
        // Password and email change routes (require session)
        .route("/auth/change-password", post(auth_change_password))
        .route("/auth/change-email", post(auth_change_email))
        .route(
            "/auth/confirm-email-change",
            post(auth_confirm_email_change),
        )
}
