use crate::validator::string_validator::validate_not_blank;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchPostsRequest {
    #[validate(length(
        min = 1,
        max = 200,
        message = "Query must be between 1 and 200 characters"
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub query: String,

    #[validate(range(min = 1, message = "Page must be greater than 0"))]
    pub page: u32,

    #[validate(range(min = 1, max = 20, message = "Page size must be between 1 and 20"))]
    pub page_size: u32,
}
