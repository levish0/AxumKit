use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP setup response
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpSetupResponse {
    /// QR code PNG image (Base64 encoded)
    pub qr_code_base64: String,
    /// otpauth:// URI (for manual entry)
    pub qr_code_uri: String,
}

impl IntoResponse for TotpSetupResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
