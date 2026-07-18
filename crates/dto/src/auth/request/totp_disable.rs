use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request to disable TOTP
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for disabling TOTP.")]
pub struct TotpDisableRequest {
    /// Current TOTP code (6 digits) or backup code (8 characters)
    #[validate(length(min = 6, max = 8, message = "Code must be 6-8 characters"))]
    pub code: String,
}
