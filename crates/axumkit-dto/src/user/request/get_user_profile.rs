use crate::validator::string_validator::validate_not_blank;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
#[into_params(parameter_in = Query)]
pub struct GetUserProfileRequest {
    #[validate(length(
        min = 3,
        max = 20,
        message = "Handle must be between 3 and 20 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub handle: String,
}
