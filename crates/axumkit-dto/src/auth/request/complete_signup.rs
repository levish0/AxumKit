use crate::validator::string_validator::validate_not_blank;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// OAuth pending signup completion request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CompleteSignupRequest {
    /// Pending signup token (returned during OAuth sign-in)
    #[validate(length(min = 1, message = "Pending token is required"))]
    pub pending_token: String,

    /// User handle (unique identifier)
    #[validate(length(
        min = 3,
        max = 20,
        message = "Handle must be between 3 and 20 characters"
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub handle: String,

    /// Display name
    #[validate(length(
        min = 1,
        max = 50,
        message = "Display name must be between 1 and 50 characters"
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub display_name: String,
}
