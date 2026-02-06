use crate::validator::string_validator::validate_not_blank;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// OAuth pending signup 완료 요청
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CompleteSignupRequest {
    /// Pending signup 토큰 (OAuth 로그인 시 반환됨)
    #[validate(length(min = 1, message = "Pending token is required"))]
    pub pending_token: String,

    /// 사용자 핸들 (고유 식별자)
    #[validate(length(
        min = 3,
        max = 20,
        message = "Handle must be between 3 and 20 characters"
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub handle: String,
}
