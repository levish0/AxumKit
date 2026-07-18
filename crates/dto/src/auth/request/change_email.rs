use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for starting an email change.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for starting an email change.")]
pub struct ChangeEmailRequest {
    /// Current password (for identity verification). No length policy is applied since it is
    /// only checked against the hash (only the hashing util's byte cap applies). An empty
    /// value simply fails as a hash mismatch.
    pub password: String,

    /// New email address
    #[validate(
        email(message = "Invalid email format."),
        length(max = 254, message = "Email must not exceed 254 characters.")
    )]
    pub new_email: String,
}
