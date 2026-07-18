use crate::validator::string_validator::{
    validate_display_name, validate_handle, validate_not_blank,
};
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
/// Request payload for update my profile request.
pub struct UpdateMyProfileRequest {
    /// Unique handle used in URLs and mentions.
    ///
    /// - Length: 4–15 characters
    /// - ASCII alphanumeric and underscores only
    /// - Cannot start or end with an underscore, no consecutive underscores
    /// - Cannot be a reserved word
    #[schema(
        min_length = 4,
        max_length = 15,
        pattern = "^[a-zA-Z0-9][a-zA-Z0-9_]*[a-zA-Z0-9]$",
        example = "john_doe"
    )]
    #[validate(length(
        min = 4,
        max = 15,
        message = "Handle must be between 4 and 15 characters."
    ))]
    #[validate(custom(function = "validate_handle"))]
    pub handle: Option<String>,

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

    #[serde(default, with = "serde_with::rust::double_option")]
    #[schema(nullable = true, max_length = 160)]
    #[validate(length(max = 160, message = "Bio cannot exceed 160 characters."))]
    #[validate(custom(function = "validate_not_blank"))]
    pub bio: Option<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::UpdateMyProfileRequest;
    use validator::Validate;

    #[test]
    fn distinguishes_missing_null_and_string_bio() {
        let missing: UpdateMyProfileRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(missing.bio, None);

        let null: UpdateMyProfileRequest = serde_json::from_str(r#"{"bio":null}"#).unwrap();
        assert_eq!(null.bio, Some(None));

        let value: UpdateMyProfileRequest = serde_json::from_str(r#"{"bio":"hello"}"#).unwrap();
        assert_eq!(value.bio, Some(Some("hello".to_string())));
    }

    #[test]
    fn validates_bio_only_when_string_is_present() {
        let delete_bio: UpdateMyProfileRequest = serde_json::from_str(r#"{"bio":null}"#).unwrap();
        assert!(delete_bio.validate().is_ok());

        let blank_bio: UpdateMyProfileRequest = serde_json::from_str(r#"{"bio":"   "}"#).unwrap();
        assert!(blank_bio.validate().is_err());
    }
}
