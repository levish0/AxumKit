use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    #[validate(email)]
    pub email: String,
    #[validate(length(
        min = 6,
        max = 20,
        message = "Password must be between 6 and 20 characters."
    ))]
    pub password: String,
    /// 로그인 유지 여부 (체크 시 30일, 미체크 시 브라우저 닫으면 만료)
    #[serde(default)]
    #[schema(example = false)]
    pub remember_me: bool,
}
