use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP Setup 응답
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpSetupResponse {
    /// QR 코드 PNG 이미지 (Base64 인코딩)
    pub qr_code_base64: String,
    /// otpauth:// URI (수동 입력용)
    pub qr_code_uri: String,
}

impl IntoResponse for TotpSetupResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
