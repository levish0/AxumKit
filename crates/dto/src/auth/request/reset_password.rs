use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordRequest {
    /// Password reset token (?token= value from the email link)
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
