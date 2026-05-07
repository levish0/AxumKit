use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP required response during login (202 Accepted)
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpRequiredResponse {
    /// Temporary token for TOTP verification
    pub temp_token: String,
}

impl IntoResponse for TotpRequiredResponse {
    fn into_response(self) -> Response {
        (StatusCode::ACCEPTED, Json(self)).into_response()
    }
}
