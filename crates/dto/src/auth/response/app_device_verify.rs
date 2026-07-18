use axum::Json;
use axum::http::{HeaderValue, StatusCode, header::CACHE_CONTROL};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Result of a native-app new-device confirmation.
///
/// The browser flow sets two cookies (session + device) on confirm; an app has no cookie jar, so it
/// receives both opaque tokens in the body: `token` is the session token to replay as
/// `Authorization: Bearer <token>`, and `device_token` is the device-recognition token to store and
/// present in the `X-Device-Token` header on subsequent logins so this device is not re-challenged.
#[derive(Debug, Serialize, ToSchema)]
#[schema(
    description = "Session and device tokens returned to a native-app client after new-device verification."
)]
pub struct AppDeviceVerifyResponse {
    /// Opaque session token. Send it back verbatim in the `Authorization: Bearer` header.
    pub token: String,
    /// Opaque device-recognition token. Store it and send it in the `X-Device-Token` header on
    /// future logins to skip the new-device email challenge on this device.
    pub device_token: String,
}

impl AppDeviceVerifyResponse {
    pub fn new(token: String, device_token: String) -> Self {
        Self {
            token,
            device_token,
        }
    }
}

impl IntoResponse for AppDeviceVerifyResponse {
    fn into_response(self) -> Response {
        let mut response = (StatusCode::OK, Json(self)).into_response();
        // Tokens must not be cached by the client or any intermediary
        // (OWASP / RFC 6749 §5.1 token-endpoint guidance).
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response
    }
}
