use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP 활성화 응답 (백업 코드 반환)
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpEnableResponse {
    /// 백업 코드 목록 (10개, 8자리 영숫자)
    pub backup_codes: Vec<String>,
}

impl IntoResponse for TotpEnableResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
