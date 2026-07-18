use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Shared request body for board post moderation actions (pin/unpin/lock/unlock).
/// Each action records a moderation-log entry, so a reason is required.
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct BoardPostModerationRequest {
    pub post_id: Uuid,
    #[validate(length(
        min = 1,
        max = 500,
        message = "Reason must be between 1 and 500 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub reason: String,
}
