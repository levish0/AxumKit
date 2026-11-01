use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::{files::contracts::FileOutput, pagination::PaginationQuery},
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct FilePageOutput {
    pub items: Vec<FileOutput>,
    pub next_cursor: Option<String>,
}

#[utoipa::path(get, path="/api/v1/organizations/{organization_id}/files", tag="Files", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path), PaginationQuery), responses((status=200,body=FilePageOutput)))]
pub async fn list_files(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<Json<FilePageOutput>, ApiError> {
    let page = state
        .app
        .files
        .list_files(
            &actor,
            organization_id,
            query.limit,
            query.cursor.as_deref(),
        )
        .await?;
    Ok(Json(FilePageOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
