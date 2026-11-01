use crate::http::{
    HttpState, error::ApiError, extract::Session,
    v1::notifications::contracts::NotificationPreferencesOutput,
};
use axum::{Json, extract::State};

#[utoipa::path(get,path="/api/v1/account/notification-preferences",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),responses((status=200,body=NotificationPreferencesOutput)))]
pub async fn get_preferences(
    State(state): State<HttpState>,
    Session(actor): Session,
) -> Result<Json<NotificationPreferencesOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .notifications
            .get_preferences(&actor)
            .await?
            .into(),
    ))
}
