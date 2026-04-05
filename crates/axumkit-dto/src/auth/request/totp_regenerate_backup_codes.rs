use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Backup code regeneration request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct TotpRegenerateBackupCodesRequest {
    /// Current TOTP code (6 digits)
    #[validate(length(equal = 6, message = "TOTP code must be 6 digits"))]
    pub code: String,
}
