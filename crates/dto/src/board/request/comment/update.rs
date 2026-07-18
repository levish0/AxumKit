use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateBoardCommentRequest {
    pub comment_id: Uuid,
    #[validate(length(
        min = 1,
        max = 40000,
        message = "Content must be between 1 and 40000 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub content: String,
}
