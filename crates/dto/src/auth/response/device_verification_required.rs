use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Response returned when new-device verification is required at login (202 Accepted).
///
/// No session is issued; a verification link is sent to the account email (OWASP ASVS 6.3.5).
#[derive(Debug, Serialize, ToSchema)]
#[schema(
    description = "Response body when a new-device email verification is required to finish login."
)]
pub struct DeviceVerificationRequiredResponse {
    /// Machine-readable status marker.
    pub status: String,
}

impl DeviceVerificationRequiredResponse {
    pub fn new() -> Self {
        Self {
            status: "device_verification_required".to_string(),
        }
    }
}

impl Default for DeviceVerificationRequiredResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoResponse for DeviceVerificationRequiredResponse {
    fn into_response(self) -> Response {
        (StatusCode::ACCEPTED, Json(self)).into_response()
    }
}
