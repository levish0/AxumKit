use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ConfirmEmailChangeRequest {
    /// 이메일 변경 토큰 (이메일 링크의 ?token= 값)
    #[validate(length(min = 1))]
    pub token: String,
}
