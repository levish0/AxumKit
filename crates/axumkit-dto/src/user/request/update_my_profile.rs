use crate::validator::string_validator::validate_not_blank;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateMyProfileRequest {
    #[validate(length(
        min = 1,
        max = 50,
        message = "Display name must be between 1 and 50 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub display_name: Option<String>,

    #[validate(length(max = 500, message = "Bio cannot exceed 500 characters."))]
    #[validate(custom(function = "validate_not_blank"))]
    pub bio: Option<String>,
}
