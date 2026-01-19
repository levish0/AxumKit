use crate::pagination::CursorDirection;
use axumkit_constants::ActionLogAction;
use axumkit_entity::common::ActionResourceType;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetActionLogsRequest {
    /// Cursor ID for pagination. None means get latest.
    pub cursor_id: Option<Uuid>,

    /// Cursor direction (default: Older when cursor_id is provided)
    pub cursor_direction: Option<CursorDirection>,

    /// Number of items to return (max: 100)
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100."))]
    pub limit: u64,

    /// Filter by user ID (for contributions)
    pub user_id: Option<Uuid>,

    /// Filter by resource ID
    pub resource_id: Option<Uuid>,

    /// Filter by resource type
    pub resource_type: Option<ActionResourceType>,

    /// Filter by actions
    pub actions: Option<Vec<ActionLogAction>>,
}
