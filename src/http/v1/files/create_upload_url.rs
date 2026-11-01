use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
    v1::files::contracts::PresignedTransferOutput,
};
use axum::{Json, extract::State};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(post, path="/api/v1/organizations/{organization_id}/files/{file_id}/upload-url", tag="Files", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("file_id"=Uuid,Path)), responses((status=200,body=PresignedTransferOutput),(status=403),(status=404),(status=409),(status=503)))]
pub async fn create_upload_url(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, file_id)): ApiPath<(Uuid, Uuid)>,
    axum::Extension(trace): axum::Extension<TraceContext>,
) -> Result<Json<PresignedTransferOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .files
            .create_upload_url(&actor, organization_id, file_id, trace)
            .await?
            .into(),
    ))
}
