use crate::validator::string_validator::validate_not_blank;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchUsersRequest {
    /// Search query for handle, display_name, or bio. Empty or omitted returns all users.
    #[validate(length(max = 100, message = "Query must be at most 100 characters."))]
    #[validate(custom(function = "validate_not_blank"))]
    #[serde(default)]
    pub query: Option<String>,

    #[validate(range(min = 1, message = "Page must be greater than 0"))]
    pub page: u32,

    #[validate(range(min = 1, max = 20, message = "Page size must be between 1 and 20"))]
    pub page_size: u32,
}
