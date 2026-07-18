use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Backup code regeneration response
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "Response body returned after TOTP backup codes are regenerated.")]
pub struct TotpBackupCodesResponse {
    /// Newly generated backup codes (10 codes, 8-character alphanumeric)
    pub backup_codes: Vec<String>,
}

impl IntoResponse for TotpBackupCodesResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
