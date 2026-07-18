use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request to regenerate backup codes
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for regenerating TOTP backup codes.")]
pub struct TotpRegenerateBackupCodesRequest {
    /// Current TOTP code (6 digits)
    #[validate(length(equal = 6, message = "TOTP code must be 6 digits"))]
    pub code: String,
}
