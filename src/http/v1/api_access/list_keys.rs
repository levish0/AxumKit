use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::{api_access::contracts::ApiKeyOutput, pagination::PaginationQuery},
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ApiKeyPageOutput {
    pub items: Vec<ApiKeyOutput>,
    pub next_cursor: Option<String>,
}

#[utoipa::path(get, path="/api/v1/organizations/{organization_id}/api-clients/{client_id}/keys", tag="API access", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("client_id"=Uuid,Path), PaginationQuery), responses((status=200,body=ApiKeyPageOutput)))]
pub async fn list_keys(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, client_id)): ApiPath<(Uuid, Uuid)>,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<Json<ApiKeyPageOutput>, ApiError> {
    let page = state
        .app
        .api_access
        .list_keys(
            &actor,
            organization_id,
            client_id,
            query.limit,
            query.cursor.as_deref(),
        )
        .await?;
    Ok(Json(ApiKeyPageOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
