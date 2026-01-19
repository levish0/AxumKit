use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// 로그인 시 TOTP 필요 응답 (202 Accepted)
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpRequiredResponse {
    /// TOTP 검증용 임시 토큰
    pub temp_token: String,
}

impl IntoResponse for TotpRequiredResponse {
    fn into_response(self) -> Response {
        (StatusCode::ACCEPTED, Json(self)).into_response()
    }
}
