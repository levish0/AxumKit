use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateBoardRequest {
    #[validate(length(
        min = 1,
        max = 128,
        message = "Slug must be between 1 and 128 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub slug: String,
    #[validate(length(
        min = 1,
        max = 256,
        message = "Name must be between 1 and 256 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub name: String,
    #[validate(length(max = 2000, message = "Description cannot exceed 2000 characters."))]
    pub description: Option<String>,
    pub order: Option<i32>,
}
