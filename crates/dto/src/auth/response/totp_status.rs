use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP status response
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpStatusResponse {
    /// Whether TOTP is enabled
    pub enabled: bool,
    /// TOTP activation time (only when enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_at: Option<DateTime<Utc>>,
    /// Remaining backup code count (only when enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_codes_remaining: Option<usize>,
}

impl IntoResponse for TotpStatusResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
