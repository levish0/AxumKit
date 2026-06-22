use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    #[validate(email)]
    pub email: String,
    /// Password for the account. Not length-validated here: login only verifies the
    /// candidate against the stored hash. Length policy is enforced at
    /// signup/change/reset, and a byte cap in the hashing util bounds DoS.
    pub password: String,
    /// Remember me (checked: 30 days, unchecked: expires when browser is closed)
    #[serde(default)]
    #[schema(example = false)]
    pub remember_me: bool,
}
