use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// 백업 코드 재생성 응답
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpBackupCodesResponse {
    /// 새로 생성된 백업 코드 목록 (10개, 8자리 영숫자)
    pub backup_codes: Vec<String>,
}

impl IntoResponse for TotpBackupCodesResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
