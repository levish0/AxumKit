use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ConfirmEmailChangeRequest {
    /// Email change token (?token= value from the email link)
    #[validate(length(min = 1))]
    pub token: String,
}
