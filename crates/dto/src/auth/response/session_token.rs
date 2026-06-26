use axum::Json;
use axum::http::{HeaderValue, StatusCode, header::CACHE_CONTROL};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Session token handed to a native-app (non-browser) client in the response body.
///
/// Browser clients never receive this — their session token lives only in an `HttpOnly` cookie that
/// page JavaScript cannot read, the standard XSS token-theft defense (OWASP Session Management
/// Cheat Sheet). Native apps have no DOM/XSS surface; they store this token in OS-backed secure
/// storage (Keychain/Keystore) and replay it as `Authorization: Bearer <token>` (RFC 6750).
///
/// The value is the same opaque session token a browser would receive in its cookie, so it carries
/// the same sliding/absolute lifetime and can be listed and revoked via the session-management
/// endpoints.
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "Session token for native-app clients, returned in the response body.")]
pub struct SessionTokenResponse {
    /// Opaque session token. Send it back verbatim in the `Authorization: Bearer` header.
    pub token: String,
}

impl SessionTokenResponse {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl IntoResponse for SessionTokenResponse {
    fn into_response(self) -> Response {
        let mut response = (StatusCode::OK, Json(self)).into_response();
        // A token must not be cached by the client or any intermediary
        // (OWASP / RFC 6749 §5.1 token-endpoint guidance).
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response
    }
}
