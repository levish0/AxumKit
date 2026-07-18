use crate::validator::datetime_validator::validate_future_datetime;
use crate::validator::string_validator::validate_not_blank;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for adding a member (a user XOR an IP/CIDR) to an ACL group.
pub struct AddGroupMemberRequest {
    /// Target group
    pub group_id: Uuid,
    /// User member (mutually exclusive with `ip_address`)
    pub user_id: Option<Uuid>,
    /// IP address or CIDR range member (mutually exclusive with `user_id`)
    #[schema(value_type = Option<String>, example = "192.168.1.0/24")]
    pub ip_address: Option<String>,
    /// Reason for the membership
    // 1000 matches the ban DTOs (BanUserRequest/BanIpRequest), which write the
    // same group_members.reason column — one cap for every writer.
    #[validate(length(max = 1000, message = "Reason must be at most 1000 characters."))]
    pub reason: Option<String>,
    /// Membership expiration time (None = permanent)
    // DTO-layer check so the error contract matches the ban DTOs (422); the
    // service re-checks as defense-in-depth.
    #[validate(custom(function = "validate_future_datetime"))]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for removing a member from an ACL group.
pub struct RemoveGroupMemberRequest {
    /// ID of the membership row to remove
    pub member_id: Uuid,
    #[validate(length(
        min = 1,
        max = 1000,
        message = "Reason must be between 1 and 1000 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub reason: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
/// Request payload for listing an ACL group's active members.
pub struct ListGroupMembersRequest {
    /// Group whose members to list
    pub group_id: Uuid,
    /// Cursor: return members older than this member id (newest-first list).
    pub cursor_id: Option<Uuid>,
    /// Page size — required, matching every other list endpoint's contract.
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100."))]
    pub limit: u64,
}
