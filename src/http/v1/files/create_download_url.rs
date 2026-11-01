use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::files::contracts::PresignedTransferOutput,
};
use axum::{Json, extract::State};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateDownloadUrlInput {
    pub method: DownloadMethod,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum DownloadMethod {
    Get,
    Head,
}

#[utoipa::path(post, path="/api/v1/organizations/{organization_id}/files/{file_id}/download-url", tag="Files", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("file_id"=Uuid,Path)), request_body=CreateDownloadUrlInput, responses((status=200,body=PresignedTransferOutput),(status=404),(status=503)))]
pub async fn create_download_url(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, file_id)): ApiPath<(Uuid, Uuid)>,
    ApiJson(input): ApiJson<CreateDownloadUrlInput>,
) -> Result<Json<PresignedTransferOutput>, ApiError> {
    let method = match input.method {
        DownloadMethod::Get => application::files::DownloadMethod::Get,
        DownloadMethod::Head => application::files::DownloadMethod::Head,
    };
    Ok(Json(
        state
            .app
            .files
            .create_download_url(&actor, organization_id, file_id, method)
            .await?
            .into(),
    ))
}
