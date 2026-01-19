use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// TOTP 상태 응답
#[derive(Debug, Serialize, ToSchema)]
pub struct TotpStatusResponse {
    /// TOTP 활성화 여부
    pub enabled: bool,
    /// TOTP 활성화 시각 (활성화된 경우만)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_at: Option<DateTime<Utc>>,
    /// 남은 백업 코드 수 (활성화된 경우만)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_codes_remaining: Option<usize>,
}

impl IntoResponse for TotpStatusResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
