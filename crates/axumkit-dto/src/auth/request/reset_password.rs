use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordRequest {
    /// 비밀번호 재설정 토큰 (이메일 링크의 ?token= 값)
    #[validate(length(min = 1))]
    pub token: String,

    /// 새 비밀번호
    #[validate(length(
        min = 6,
        max = 20,
        message = "Password must be between 6 and 20 characters."
    ))]
    pub new_password: String,
}
