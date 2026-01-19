use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// 백업 코드 재생성 요청
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TotpRegenerateBackupCodesRequest {
    /// 현재 TOTP 코드 (6자리)
    #[validate(length(equal = 6, message = "TOTP code must be 6 digits"))]
    pub code: String,
}
