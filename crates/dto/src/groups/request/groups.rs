use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for creating an ACL group.
pub struct CreateGroupRequest {
    /// Unique group name (e.g. "vpn-ranges")
    #[validate(length(
        min = 1,
        max = 64,
        message = "Name must be between 1 and 64 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub name: String,
    /// Human-readable group description
    #[validate(length(max = 500, message = "Description must be at most 500 characters."))]
    pub description: Option<String>,
    #[validate(length(
        min = 1,
        max = 500,
        message = "Reason must be between 1 and 500 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for deleting an ACL group.
pub struct DeleteGroupRequest {
    /// ID of the group to delete
    pub group_id: Uuid,
    #[validate(length(
        min = 1,
        max = 500,
        message = "Reason must be between 1 and 500 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub reason: String,
}
