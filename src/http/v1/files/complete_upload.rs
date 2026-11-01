use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
    v1::files::contracts::FileOutput,
};
use axum::{Json, extract::State};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(post, path="/api/v1/organizations/{organization_id}/files/{file_id}/complete-upload", tag="Files", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("file_id"=Uuid,Path)), responses((status=200,body=FileOutput),(status=400),(status=403),(status=404),(status=409),(status=503)))]
pub async fn complete_upload(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, file_id)): ApiPath<(Uuid, Uuid)>,
    axum::Extension(trace): axum::Extension<TraceContext>,
) -> Result<Json<FileOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .files
            .complete_upload(&actor, organization_id, file_id, trace)
            .await?
            .into(),
    ))
}
