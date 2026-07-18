use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateBoardCommentRequest {
    pub post_id: Uuid,
    /// When set, this comment is a reply. Depth is capped at 2: a reply to a reply
    /// is attached to the same thread root.
    pub parent_comment_id: Option<Uuid>,
    #[validate(length(
        min = 1,
        max = 40000,
        message = "Content must be between 1 and 40000 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub content: String,
}
