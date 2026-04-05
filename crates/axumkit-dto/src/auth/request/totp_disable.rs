use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// TOTP disable request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TotpDisableRequest {
    /// Current TOTP code (6 digits) or backup code (8 digits)
    #[validate(length(min = 6, max = 8, message = "Code must be 6-8 characters"))]
    pub code: String,
}
