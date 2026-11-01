use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
    v1::notifications::contracts::NotificationOutput,
};
use axum::{Json, extract::State};
use uuid::Uuid;

#[utoipa::path(get,path="/api/v1/notifications/{notification_id}",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("notification_id"=Uuid,Path)),responses((status=200,body=NotificationOutput)))]
pub async fn get_notification(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(notification_id): ApiPath<Uuid>,
) -> Result<Json<NotificationOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .notifications
            .get_notification(&actor, notification_id)
            .await?
            .into(),
    ))
}
