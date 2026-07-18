use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// TOTP verification request (second factor during login)
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for the second step of TOTP login.")]
pub struct TotpVerifyRequest {
    /// Temporary token received at login
    pub temp_token: String,
    /// TOTP code (6 digits) or backup code (8 characters)
    #[validate(length(min = 6, max = 8, message = "Code must be 6-8 characters"))]
    pub code: String,
}
