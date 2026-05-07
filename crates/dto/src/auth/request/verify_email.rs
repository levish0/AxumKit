use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyEmailRequest {
    /// Email verification token
    #[validate(length(min = 1))]
    pub token: String,
}
