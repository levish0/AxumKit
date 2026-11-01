use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::{api_access::contracts::ApiClientOutput, pagination::PaginationQuery},
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ApiClientPageOutput {
    pub items: Vec<ApiClientOutput>,
    pub next_cursor: Option<String>,
}

#[utoipa::path(get, path="/api/v1/organizations/{organization_id}/api-clients", tag="API access", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path), PaginationQuery), responses((status=200,body=ApiClientPageOutput)))]
pub async fn list_clients(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<Json<ApiClientPageOutput>, ApiError> {
    let page = state
        .app
        .api_access
        .list_clients(
            &actor,
            organization_id,
            query.limit,
            query.cursor.as_deref(),
        )
        .await?;
    Ok(Json(ApiClientPageOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
