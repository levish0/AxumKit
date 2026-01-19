use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// TOTP 검증 요청 (로그인 시 2단계 인증)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TotpVerifyRequest {
    /// 로그인 시 받은 임시 토큰
    pub temp_token: String,
    /// TOTP 코드 (6자리) 또는 백업 코드 (8자리)
    #[validate(length(min = 6, max = 8, message = "Code must be 6-8 characters"))]
    pub code: String,
}
