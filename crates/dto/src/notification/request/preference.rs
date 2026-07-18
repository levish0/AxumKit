use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
/// Request payload for update notification preference request.
pub struct UpdateNotificationPreferenceRequest {
    pub email_enabled: Option<bool>,
    pub push_enabled: Option<bool>,
}
