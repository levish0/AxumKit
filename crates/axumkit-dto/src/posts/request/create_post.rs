use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreatePostRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "Title must be between 1 and 200 characters"
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub title: String,
    #[validate(length(
        min = 1,
        max = 40000,
        message = "Content must be between 1 and 40000 characters"
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub content: String,
}
