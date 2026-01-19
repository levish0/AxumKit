use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// TOTP 비활성화 요청
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TotpDisableRequest {
    /// 현재 TOTP 코드 (6자리) 또는 백업 코드 (8자리)
    #[validate(length(min = 6, max = 8, message = "Code must be 6-8 characters"))]
    pub code: String,
}
