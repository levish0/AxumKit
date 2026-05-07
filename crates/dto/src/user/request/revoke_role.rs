use crate::validator::string_validator::validate_not_blank;
use entity::common::Role;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for revoke role request.
pub struct RevokeRoleRequest {
    pub user_id: Uuid,
    pub role: Role,
    #[validate(length(
        min = 1,
        max = 1000,
        message = "Reason must be between 1 and 1000 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub reason: String,
}
