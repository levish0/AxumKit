use crate::http::{HttpState, error::ApiError, extract::Session};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ReadBatchOutput {
    pub updated: u64,
}

#[utoipa::path(post,path="/api/v1/notifications/read-batch",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),responses((status=200,body=ReadBatchOutput)))]
pub async fn mark_read_batch(
    State(state): State<HttpState>,
    Session(actor): Session,
) -> Result<Json<ReadBatchOutput>, ApiError> {
    Ok(Json(ReadBatchOutput {
        updated: state.app.notifications.mark_read_batch(&actor).await?,
    }))
}
