use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP status response
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "Response body describing the current TOTP status.")]
pub struct TotpStatusResponse {
    /// Whether TOTP is enabled
    pub enabled: bool,
    /// When TOTP was enabled (only if enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_at: Option<DateTime<Utc>>,
    /// Number of remaining backup codes (only if enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_codes_remaining: Option<usize>,
}

impl IntoResponse for TotpStatusResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
