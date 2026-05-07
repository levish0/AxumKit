use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangeEmailRequest {
    /// Current password (for identity verification)
    #[validate(length(min = 1))]
    pub password: String,

    /// New email address
    #[validate(email(message = "Invalid email format."))]
    pub new_email: String,
}
