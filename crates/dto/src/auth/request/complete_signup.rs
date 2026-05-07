use crate::validator::string_validator::{
    validate_display_name, validate_handle, validate_not_blank,
};
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// OAuth pending signup completion request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[schema(
    description = "Request body for completing an OAuth signup after a provider login for a new user."
)]
pub struct CompleteSignupRequest {
    /// Pending signup token (returned during OAuth sign-in)
    #[validate(length(min = 1, message = "Pending token is required"))]
    pub pending_token: String,

    /// Unique user handle.
    ///
    /// - Length: 4–15 characters
    /// - Allowed characters: `a-z`, `A-Z`, `0-9`, `_`
    /// - Cannot start or end with `_`
    /// - No consecutive underscores (`__`)
    /// - Reserved words are not allowed (e.g. `admin`, `support`, `help`, `system`)
    #[schema(
        min_length = 4,
        max_length = 15,
        pattern = "^[a-zA-Z0-9][a-zA-Z0-9_]*[a-zA-Z0-9]$",
        example = "john_doe"
    )]
    #[validate(length(
        min = 4,
        max = 15,
        message = "Handle must be between 4 and 15 characters"
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    #[validate(custom(function = "validate_handle"))]
    pub handle: String,

    /// Display name shown in the UI.
    ///
    /// - Length: 1-50 characters
    /// - Unicode letters, spaces, and punctuation are permitted
    /// - Emoji, control characters, and invisible Unicode are not allowed
    #[schema(min_length = 1, max_length = 50, example = "John Doe")]
    #[validate(length(
        min = 1,
        max = 50,
        message = "Display name must be between 1 and 50 characters"
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    #[validate(custom(function = "validate_display_name"))]
    pub display_name: String,
}
