use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP enable response (returns backup codes)
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpEnableResponse {
    /// List of backup codes (10 codes, 8-character alphanumeric)
    pub backup_codes: Vec<String>,
}

impl IntoResponse for TotpEnableResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
