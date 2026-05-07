use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Backup code regeneration response
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpBackupCodesResponse {
    /// Newly generated list of backup codes (10 codes, 8-character alphanumeric)
    pub backup_codes: Vec<String>,
}

impl IntoResponse for TotpBackupCodesResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
