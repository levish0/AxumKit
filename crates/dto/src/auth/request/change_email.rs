use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangeEmailRequest {
    /// Current password (for identity verification). Not length-validated here: it
    /// is only a verification candidate (the hashing util's byte cap still bounds
    /// it), and an empty value simply fails as a hash mismatch.
    pub password: String,

    /// New email address
    #[validate(email(message = "Invalid email format."))]
    pub new_email: String,
}
