use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for completing an email signup.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for completing an email signup.")]
pub struct VerifyEmailRequest {
    /// Email verification token
    #[validate(length(min = 1))]
    pub token: String,
}
