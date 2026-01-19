use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangeEmailRequest {
    /// 현재 비밀번호 (본인 확인용)
    #[validate(length(min = 1))]
    pub password: String,

    /// 새 이메일 주소
    #[validate(email(message = "Invalid email format."))]
    pub new_email: String,
}
