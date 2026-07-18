use crate::validator::string_validator::validate_not_blank;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ParseBoardRequest {
    /// Board post or comment markup to render. Bounded by the post/comment body limit.
    #[validate(length(min = 1, max = 40000))]
    #[validate(custom(function = "validate_not_blank"))]
    pub content: String,
}
