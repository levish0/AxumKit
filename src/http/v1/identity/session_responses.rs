use super::contracts::{LoginOutput, SessionTransport};
use crate::http::{HttpState, cookies, error::ApiError, middleware::require_browser_origin};
use application::identity::{LoginOutcome, SessionCredentials};
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

pub(super) fn login_response(
    state: &HttpState,
    outcome: LoginOutcome,
    transport: SessionTransport,
) -> Response {
    match outcome {
        LoginOutcome::Session(session) => session_response(state, session, transport),
        LoginOutcome::MfaRequired { challenge } => {
            Json(LoginOutput::MfaRequired { challenge }).into_response()
        }
    }
}
pub(super) fn session_response(
    state: &HttpState,
    session: SessionCredentials,
    transport: SessionTransport,
) -> Response {
    let token = matches!(transport, SessionTransport::Bearer).then(|| session.token.clone());
    let mut response = Json(LoginOutput::Authenticated {
        session_id: session.session_id,
        expires_at: session.expires_at,
        token,
    })
    .into_response();
    if matches!(transport, SessionTransport::Cookie) {
        cookies::set_cookie(
            &mut response,
            &state.settings,
            state.settings.session_cookie_name(),
            session.token,
            (session.expires_at - chrono::Utc::now()).num_seconds(),
        );
    }
    response
}
pub(super) fn cleared_session_response(state: &HttpState) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    cookies::clear_session(&mut response, &state.settings);
    response
}
pub(super) fn validate_transport(
    state: &HttpState,
    headers: &HeaderMap,
    transport: SessionTransport,
) -> Result<(), ApiError> {
    if matches!(transport, SessionTransport::Cookie) {
        require_browser_origin(state, headers)?;
    }
    Ok(())
}
