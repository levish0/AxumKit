use super::{
    begin_totp, change_password, complete_mfa, confirm_totp, deactivate_account, disable_totp,
    get_account, list_sessions, login, oidc, regenerate_recovery_codes, request_reset,
    request_verification, reset_password, revoke_all_sessions, revoke_session, rotate_session,
    set_initial_password, signup, verify_email,
};
use crate::http::HttpState;

pub(in crate::http::v1) fn routes() -> axum::Router<HttpState> {
    axum::Router::new()
        .route("/auth/signup", axum::routing::post(signup::signup))
        .route(
            "/auth/verification",
            axum::routing::post(request_verification::request_verification),
        )
        .route(
            "/auth/verification/confirm",
            axum::routing::post(verify_email::verify_email),
        )
        .route("/auth/login", axum::routing::post(login::login))
        .route("/auth/mfa", axum::routing::post(complete_mfa::complete_mfa))
        .route(
            "/auth/password/reset",
            axum::routing::post(request_reset::request_reset),
        )
        .route(
            "/auth/password/reset/confirm",
            axum::routing::post(reset_password::reset_password),
        )
        .route(
            "/account",
            axum::routing::get(get_account::get_account)
                .delete(deactivate_account::deactivate_account),
        )
        .route(
            "/account/password",
            axum::routing::put(change_password::change_password),
        )
        .route(
            "/account/password/initial",
            axum::routing::post(set_initial_password::set_initial_password),
        )
        .route(
            "/account/totp",
            axum::routing::post(begin_totp::begin_totp).delete(disable_totp::disable_totp),
        )
        .route(
            "/account/totp/confirm",
            axum::routing::post(confirm_totp::confirm_totp),
        )
        .route(
            "/account/recovery-codes",
            axum::routing::post(regenerate_recovery_codes::regenerate_recovery_codes),
        )
        .route(
            "/sessions",
            axum::routing::get(list_sessions::list_sessions)
                .delete(revoke_all_sessions::revoke_all_sessions),
        )
        .route(
            "/sessions/{session_id}",
            axum::routing::delete(revoke_session::revoke_session),
        )
        .route(
            "/sessions/current/rotate",
            axum::routing::post(rotate_session::rotate_session),
        )
        .route(
            "/auth/oidc/{provider}",
            axum::routing::post(oidc::begin_login::begin_login),
        )
        .route(
            "/auth/oidc/{provider}/callback",
            axum::routing::get(oidc::callback::callback),
        )
        .route(
            "/auth/oidc/{provider}/native",
            axum::routing::post(oidc::begin_native_login::begin_native_login),
        )
        .route(
            "/auth/oidc/{provider}/native/complete",
            axum::routing::post(oidc::complete_native_login::complete_native_login),
        )
        .route(
            "/account/identities",
            axum::routing::get(oidc::list_identities::list_identities),
        )
        .route(
            "/account/identities/providers/{provider}",
            axum::routing::post(oidc::begin_link::begin_link),
        )
        .route(
            "/account/identities/{identity_id}",
            axum::routing::delete(oidc::unlink_identity::unlink_identity),
        )
}
