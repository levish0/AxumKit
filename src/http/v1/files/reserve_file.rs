use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::files::contracts::FileOutput,
    validation,
};
use application::files::ReserveFile;
use axum::{Json, extract::State, http::StatusCode};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct ReserveFileInput {
    #[validate(custom(function = "validation::filename"))]
    /// Display filename: 1 to 255 UTF-8 bytes, without path separators or control characters.
    pub filename: String,
    #[validate(custom(function = "validation::media_type"))]
    #[schema(max_length = 120)]
    pub media_type: Option<String>,
    pub byte_size: u64,
}

#[utoipa::path(post, path="/api/v1/organizations/{organization_id}/files", tag="Files", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path)), request_body=ReserveFileInput, responses((status=201,body=FileOutput)))]
pub async fn reserve_file(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    axum::Extension(trace): axum::Extension<TraceContext>,
    ApiJson(input): ApiJson<ReserveFileInput>,
) -> Result<(StatusCode, Json<FileOutput>), ApiError> {
    let file = state
        .app
        .files
        .reserve_file(
            &actor,
            organization_id,
            ReserveFile {
                filename: input.filename,
                media_type: input.media_type,
                byte_size: input.byte_size,
            },
            trace,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(file.into())))
}
