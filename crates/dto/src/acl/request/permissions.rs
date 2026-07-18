use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for replacing a group's permission grants (whole-list
/// replacement: submit the desired end state).
pub struct ReplaceAclGroupPermissionsRequest {
    /// Target group
    pub group_id: Uuid,
    /// Full desired permission list (codenames, e.g. "board:pin_post")
    #[validate(length(max = 100, message = "At most 100 permissions per group."))]
    pub permissions: Vec<String>,
    /// Moderation-log reason
    #[validate(length(
        min = 1,
        max = 500,
        message = "Reason must be between 1 and 500 characters."
    ))]
    pub reason: String,
}
