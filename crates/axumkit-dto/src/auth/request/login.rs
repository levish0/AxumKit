use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    #[validate(email)]
    pub email: String,
    #[validate(length(
        min = 6,
        max = 20,
        message = "Password must be between 6 and 20 characters."
    ))]
    pub password: String,
    /// Remember me (checked: 30 days, unchecked: expires when browser is closed)
    #[serde(default)]
    #[schema(example = false)]
    pub remember_me: bool,
}
