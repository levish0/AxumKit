use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateUserRequest {
    #[schema(example = "user@example.com")]
    #[validate(email)]
    pub email: String,
    #[validate(length(
        min = 3,
        max = 20,
        message = "Handle must be between 3 and 20 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub handle: String,
    #[validate(length(
        min = 3,
        max = 20,
        message = "Name must be between 3 and 20 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub display_name: String,
    #[validate(length(
        min = 6,
        max = 20,
        message = "Password must be between 6 and 20 characters."
    ))]
    pub password: String,
}
