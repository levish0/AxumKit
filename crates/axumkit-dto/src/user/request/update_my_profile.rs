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
    pub display_name: Option<String>,

    #[validate(length(max = 500, message = "Bio cannot exceed 500 characters."))]
    pub bio: Option<String>,
}
