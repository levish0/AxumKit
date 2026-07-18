use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for completing a password reset.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for completing a password reset.")]
pub struct ResetPasswordRequest {
    /// Password reset token (the ?token= value from the email link)
    #[validate(length(min = 1))]
    pub token: String,

    /// New password
    #[validate(length(
        min = 12,
        max = 128,
        message = "Password must be between 12 and 128 characters."
    ))]
    pub new_password: String,
}
