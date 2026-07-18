use crate::validator::string_validator::validate_not_blank;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetBoardBySlugRequest {
    #[validate(length(
        min = 1,
        max = 128,
        message = "Slug must be between 1 and 128 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub slug: String,
}
