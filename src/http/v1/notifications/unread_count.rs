use crate::http::{HttpState, error::ApiError, extract::Session};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct UnreadOutput {
    pub unread_count: u64,
}

#[utoipa::path(get,path="/api/v1/notifications/unread-count",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),responses((status=200,body=UnreadOutput)))]
pub async fn unread_count(
    State(state): State<HttpState>,
    Session(actor): Session,
) -> Result<Json<UnreadOutput>, ApiError> {
    Ok(Json(UnreadOutput {
        unread_count: state.app.notifications.unread_count(&actor).await?,
    }))
}
