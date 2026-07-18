use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP enable response (returns backup codes)
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "Response body returned after TOTP is enabled.")]
pub struct TotpEnableResponse {
    /// Backup codes (10 codes, 8-character alphanumeric)
    pub backup_codes: Vec<String>,
}

impl IntoResponse for TotpEnableResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
