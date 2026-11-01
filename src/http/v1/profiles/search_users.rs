use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::{pagination::SearchQuery, profiles::contracts::ProfileOutput},
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ProfileSearchOutput {
    pub items: Vec<ProfileOutput>,
    pub page: usize,
    pub per_page: usize,
    pub total_in_window: usize,
    pub search_window_limit: usize,
    pub has_next_page: bool,
}

#[utoipa::path(get,path="/api/v1/organizations/{organization_id}/users/search",tag="Profiles",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),SearchQuery),responses((status=200,body=ProfileSearchOutput)))]
pub async fn search_users(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<SearchQuery>,
) -> Result<Json<ProfileSearchOutput>, ApiError> {
    let page = state
        .app
        .profiles
        .search_users(
            &actor,
            organization_id,
            &query.q,
            query.page,
            query.per_page,
        )
        .await?;
    Ok(Json(ProfileSearchOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        page: page.page,
        per_page: page.per_page,
        total_in_window: page.total_in_window,
        search_window_limit: page.search_window_limit,
        has_next_page: page.has_next_page,
    }))
}
