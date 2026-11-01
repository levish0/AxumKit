use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, Session},
    v1::notifications::contracts::NotificationPreferencesOutput,
};
use application::notifications::NotificationPreferences;
use axum::{Json, extract::State};
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NotificationPreferencesInput {
    pub post_comments: bool,
    pub push_enabled: bool,
}
impl From<NotificationPreferencesInput> for NotificationPreferences {
    fn from(value: NotificationPreferencesInput) -> Self {
        Self {
            post_comments: value.post_comments,
            push_enabled: value.push_enabled,
        }
    }
}

#[utoipa::path(put,path="/api/v1/account/notification-preferences",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),request_body=NotificationPreferencesOutput,responses((status=200,body=NotificationPreferencesOutput)))]
pub async fn save_preferences(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiJson(input): ApiJson<NotificationPreferencesInput>,
) -> Result<Json<NotificationPreferencesOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .notifications
            .save_preferences(&actor, input.into())
            .await?
            .into(),
    ))
}
