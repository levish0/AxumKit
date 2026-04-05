use crate::validator::string_validator::{validate_handle, validate_not_blank};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, IntoParams, Validate)]
#[into_params(parameter_in = Path)]
/// Request payload for check handle available path.
pub struct CheckHandleAvailablePath {
    #[validate(length(
        min = 4,
        max = 15,
        message = "Handle must be between 4 and 15 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    #[validate(custom(function = "validate_handle"))]
    pub handle: String,
}
