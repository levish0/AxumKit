use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::files::contracts::FileOutput,
};
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, ToSchema)]
pub struct FileSearchOutput {
    pub items: Vec<FileOutput>,
}

#[derive(Deserialize, IntoParams, Validate)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct FileSearchQuery {
    #[validate(length(min = 1, max = 200))]
    #[param(min_length = 1, max_length = 200)]
    pub q: String,
}

#[utoipa::path(get, path="/api/v1/organizations/{organization_id}/files/search", tag="Files", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),FileSearchQuery), responses((status=200,body=FileSearchOutput)))]
pub async fn search_files(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<FileSearchQuery>,
) -> Result<Json<FileSearchOutput>, ApiError> {
    Ok(Json(FileSearchOutput {
        items: state
            .app
            .files
            .search_files(&actor, organization_id, &query.q)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    }))
}
