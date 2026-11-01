use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
};
use axum::{extract::State, http::StatusCode};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(delete, path="/api/v1/organizations/{organization_id}/files/{file_id}", tag="Files", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("file_id"=Uuid,Path)), responses((status=204)))]
pub async fn delete_file(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, file_id)): ApiPath<(Uuid, Uuid)>,
    axum::Extension(trace): axum::Extension<TraceContext>,
) -> Result<StatusCode, ApiError> {
    state
        .app
        .files
        .delete_file(&actor, organization_id, file_id, trace)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
