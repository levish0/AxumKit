use crate::pagination::CursorDirection;
use axumkit_constants::ModerationAction;
use axumkit_entity::common::ModerationResourceType;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListModerationLogsRequest {
    pub cursor_id: Option<Uuid>,
    pub cursor_direction: Option<CursorDirection>,
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100."))]
    pub limit: u64,
    pub actor_id: Option<Uuid>,
    pub resource_type: Option<ModerationResourceType>,
    pub resource_id: Option<Uuid>,
    pub actions: Option<Vec<ModerationAction>>,
}
