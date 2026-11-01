use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
};
use axum::{extract::State, http::StatusCode};
use uuid::Uuid;

#[utoipa::path(put,path="/api/v1/notifications/{notification_id}/read",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("notification_id"=Uuid,Path)),responses((status=204)))]
pub async fn mark_read(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(notification_id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .app
        .notifications
        .mark_read(&actor, notification_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
