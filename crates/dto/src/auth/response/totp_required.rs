use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Response returned when TOTP is required at login (202 Accepted)
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "Response body returned when TOTP is required to finish login.")]
pub struct TotpRequiredResponse {
    /// Temporary token for TOTP verification
    pub temp_token: String,
}

impl IntoResponse for TotpRequiredResponse {
    fn into_response(self) -> Response {
        (StatusCode::ACCEPTED, Json(self)).into_response()
    }
}
