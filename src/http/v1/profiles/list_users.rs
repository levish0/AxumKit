use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::{pagination::PaginationQuery, profiles::contracts::ProfileOutput},
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ProfilePageOutput {
    pub items: Vec<ProfileOutput>,
    pub next_cursor: Option<String>,
}

#[utoipa::path(get,path="/api/v1/organizations/{organization_id}/users",tag="Profiles",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),PaginationQuery),responses((status=200,body=ProfilePageOutput)))]
pub async fn list_users(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<Json<ProfilePageOutput>, ApiError> {
    let page = state
        .app
        .profiles
        .list_users(
            &actor,
            organization_id,
            query.limit,
            query.cursor.as_deref(),
        )
        .await?;
    Ok(Json(ProfilePageOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
