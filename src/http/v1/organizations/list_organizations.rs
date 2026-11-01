use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiQuery, Session},
    v1::{organizations::contracts::OrganizationOutput, pagination::PaginationQuery},
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct OrganizationPageOutput {
    pub items: Vec<OrganizationOutput>,
    pub next_cursor: Option<String>,
}

#[utoipa::path(get, path="/api/v1/organizations", tag="Organizations", security(("cookieAuth"=[]),("bearerAuth"=[])), params(PaginationQuery), responses((status=200,body=OrganizationPageOutput)))]
pub async fn list_organizations(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<Json<OrganizationPageOutput>, ApiError> {
    let page = state
        .app
        .organizations
        .list_organizations(&actor, query.limit, query.cursor.as_deref())
        .await?;
    Ok(Json(OrganizationPageOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
