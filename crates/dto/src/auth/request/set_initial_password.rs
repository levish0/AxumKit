use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for setting the first password on an OAuth-only account.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for setting the first password on an OAuth-only account.")]
pub struct SetInitialPasswordRequest {
    /// New password
    #[validate(length(
        min = 12,
        max = 128,
        message = "Password must be between 12 and 128 characters."
    ))]
    pub new_password: String,
}
