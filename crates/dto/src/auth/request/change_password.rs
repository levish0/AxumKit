use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    /// Current password. Not length-validated here: it is only a verification
    /// candidate (the hashing util's byte cap still bounds it), and an empty value
    /// simply fails as a hash mismatch.
    pub current_password: String,

    /// New password
    #[validate(length(
        min = 12,
        max = 128,
        message = "Password must be between 12 and 128 characters."
    ))]
    pub new_password: String,
}
