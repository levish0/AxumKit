use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
    v1::files::contracts::FileOutput,
};
use axum::{Json, extract::State};
use uuid::Uuid;

#[utoipa::path(get, path="/api/v1/organizations/{organization_id}/files/{file_id}", tag="Files", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("file_id"=Uuid,Path)), responses((status=200,body=FileOutput)))]
pub async fn get_file(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, file_id)): ApiPath<(Uuid, Uuid)>,
) -> Result<Json<FileOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .files
            .get_file(&actor, organization_id, file_id)
            .await?
            .into(),
    ))
}
