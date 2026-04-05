use crate::validator::string_validator::{validate_display_name, validate_not_blank};
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
/// Request payload for update my profile request.
pub struct UpdateMyProfileRequest {
    /// Display name shown in the UI.
    ///
    /// - Length: 1–50 characters
    /// - Unicode letters, spaces, and punctuation are permitted
    /// - Emoji, control characters, and invisible Unicode are not allowed
    #[schema(min_length = 1, max_length = 50, example = "John Doe")]
    #[validate(length(
        min = 1,
        max = 50,
        message = "Display name must be between 1 and 50 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    #[validate(custom(function = "validate_display_name"))]
    pub display_name: Option<String>,

    #[validate(length(max = 500, message = "Bio cannot exceed 500 characters."))]
    #[validate(custom(function = "validate_not_blank"))]
    pub bio: Option<String>,
}
