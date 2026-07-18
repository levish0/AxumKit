use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for starting a password reset.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for starting a password reset.")]
pub struct ForgotPasswordRequest {
    /// Email address requesting the password reset
    #[validate(
        email,
        length(max = 254, message = "Email must not exceed 254 characters.")
    )]
    pub email: String,
}
