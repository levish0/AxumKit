use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for deleting the current account.
///
/// Account deletion is a sensitive action and requires re-authentication (OWASP ASVS 7.5.1).
/// The service picks the factor to verify from the account itself:
/// - password accounts → `password` is required and verified inline;
/// - OAuth-only accounts with TOTP enabled → `totp_code` (6-digit TOTP or 8-char backup) is required;
/// - OAuth-only accounts with no TOTP → neither field applies; a confirmation email is sent instead.
#[derive(Debug, Default, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for deleting the current account (re-authentication).")]
pub struct DeleteMyAccountRequest {
    /// Current password. Required for accounts that have a password set.
    pub password: Option<String>,

    /// TOTP code (6 digits) or backup code (8 chars). Required for OAuth-only accounts
    /// that have TOTP enabled.
    pub totp_code: Option<String>,
}
