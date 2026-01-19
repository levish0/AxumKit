use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// TOTP 활성화 요청 (setup 후 첫 코드 검증)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TotpEnableRequest {
    /// 인증 앱에서 생성한 6자리 TOTP 코드
    #[validate(length(equal = 6, message = "TOTP code must be 6 digits"))]
    pub code: String,
}
