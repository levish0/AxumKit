use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for changing the current password.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for changing the current password.")]
pub struct ChangePasswordRequest {
    /// Current password. No length policy is applied since it is only checked against the
    /// hash (only the hashing util's byte cap applies). An empty value simply fails as a
    /// hash mismatch.
    pub current_password: String,

    /// New password
    #[validate(length(
        min = 12,
        max = 128,
        message = "Password must be between 12 and 128 characters."
    ))]
    pub new_password: String,
}
