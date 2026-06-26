use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

/// Headers the API gateway (APISIX) reads from the `/auth/check` forward-auth response to learn
/// the request's identity for **rate limiting** (e.g. per-user vs. per-IP buckets). The backend
/// does not consume these for authorization — the session credential remains the sole authz
/// authority (see `extractors::session`). Single source of truth for the gateway-facing contract.
pub const AUTH_STATUS_HEADER: &str = "x-auth-status";
pub const AUTH_USER_ID_HEADER: &str = "x-auth-user-id";
pub const AUTH_SESSION_ID_HEADER: &str = "x-auth-session-id";
pub const AUTH_MANAGEMENT_ID_HEADER: &str = "x-auth-management-id";

/// `X-Auth-Status` values: a request resolved to a valid session vs. one without.
pub const AUTH_STATUS_AUTHENTICATED: &str = "authenticated";
pub const AUTH_STATUS_ANONYMOUS: &str = "anonymous";

/// Identity advertised to the gateway when a session is valid.
pub struct AuthCheckIdentity {
    pub user_id: String,
    pub session_id: String,
    pub management_id: String,
}

/// Build the `/auth/check` response: always 200, with `X-Auth-Status` plus the identity
/// headers when authenticated. The gateway forwards these upstream and buckets rate limits
/// per user; it never blocks here, so route-level authorization stays in the application.
pub fn create_auth_check_response(identity: Option<AuthCheckIdentity>) -> Response {
    let mut response = StatusCode::OK.into_response();
    let headers = response.headers_mut();

    let Some(identity) = identity else {
        headers.insert(
            AUTH_STATUS_HEADER,
            HeaderValue::from_static(AUTH_STATUS_ANONYMOUS),
        );
        return response;
    };

    headers.insert(
        AUTH_STATUS_HEADER,
        HeaderValue::from_static(AUTH_STATUS_AUTHENTICATED),
    );
    // user/session/management ids are UUIDs / hex hashes, so they are always valid header values.
    if let Ok(value) = HeaderValue::from_str(&identity.user_id) {
        headers.insert(AUTH_USER_ID_HEADER, value);
    }
    if let Ok(value) = HeaderValue::from_str(&identity.session_id) {
        headers.insert(AUTH_SESSION_ID_HEADER, value);
    }
    if let Ok(value) = HeaderValue::from_str(&identity.management_id) {
        headers.insert(AUTH_MANAGEMENT_ID_HEADER, value);
    }

    response
}
