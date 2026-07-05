use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for confirming an account deletion via the emailed single-use token.
///
/// Used by OAuth-only accounts without a password or TOTP factor: the token emailed to the
/// account address is the re-authentication proof.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for confirming account deletion via email token.")]
pub struct ConfirmAccountDeletionRequest {
    /// The single-use token delivered to the account's email address.
    #[validate(length(min = 1, message = "Token must not be empty."))]
    pub token: String,
}
