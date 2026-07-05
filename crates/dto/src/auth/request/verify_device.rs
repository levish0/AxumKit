use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for confirming a new-device sign-in via the emailed single-use token.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for confirming a new-device sign-in.")]
pub struct VerifyDeviceRequest {
    /// The single-use token delivered to the account's email address.
    #[validate(length(min = 1, message = "Token must not be empty."))]
    pub token: String,
}
