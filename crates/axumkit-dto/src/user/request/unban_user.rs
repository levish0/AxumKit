use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for unban user request.
pub struct UnbanUserRequest {
    pub user_id: Uuid,
    #[validate(length(
        min = 1,
        max = 1000,
        message = "Reason must be between 1 and 1000 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub reason: String,
}
