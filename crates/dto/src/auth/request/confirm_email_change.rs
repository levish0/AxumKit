use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request body for confirming an email change.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for confirming an email change.")]
pub struct ConfirmEmailChangeRequest {
    /// Email change token (the ?token= value from the email link)
    #[validate(length(min = 1))]
    pub token: String,
}
