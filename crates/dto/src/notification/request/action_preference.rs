use constants::NotificationAction;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for update action preference request.
pub struct UpdateActionPreferenceRequest {
    pub action: NotificationAction,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
/// Request payload for update action preferences bulk request.
pub struct UpdateActionPreferencesBulkRequest {
    #[validate(length(min = 1, message = "At least one preference update is required."))]
    pub updates: Vec<UpdateActionPreferenceRequest>,
}
