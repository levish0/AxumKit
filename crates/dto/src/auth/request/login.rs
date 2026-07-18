use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request body for email and password login.
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for email and password login.")]
pub struct LoginRequest {
    /// Email address used to sign in.
    #[schema(example = "user@example.com")]
    #[validate(
        email,
        length(max = 254, message = "Email must not exceed 254 characters.")
    )]
    pub email: String,
    /// Password for the account. Not length-validated here: login only verifies the
    /// candidate against the stored hash. Length policy is enforced at
    /// signup/change/reset, and a byte cap in the hashing util bounds DoS.
    pub password: String,
    /// Whether to stay signed in (30 days if checked; expires when the browser closes if not)
    #[serde(default)]
    #[schema(example = false)]
    pub remember_me: bool,
}
