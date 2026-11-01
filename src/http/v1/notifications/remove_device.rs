use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
};
use axum::{extract::State, http::StatusCode};
use uuid::Uuid;

#[utoipa::path(delete,path="/api/v1/account/push-devices/{device_id}",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("device_id"=Uuid,Path)),responses((status=204)))]
pub async fn remove_device(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(device_id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .app
        .notifications
        .remove_device(&actor, device_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
