use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for resending a pending signup verification email.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for resending a pending signup verification email.")]
pub struct ResendVerificationEmailRequest {
    /// Verification email recipient address
    #[validate(
        email,
        length(max = 254, message = "Email must not exceed 254 characters.")
    )]
    pub email: String,
}
