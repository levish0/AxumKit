use crate::service::search::search_users::service_search_users;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::search::{SearchUsersRequest, SearchUsersResponse};
use axumkit_dto::validator::query_validator::ValidatedQuery;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/search/users",
    params(SearchUsersRequest),
    responses(
        (status = 200, description = "User search results", body = SearchUsersResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Search"
)]
pub async fn search_users(
    State(state): State<AppState>,
    ValidatedQuery(payload): ValidatedQuery<SearchUsersRequest>,
) -> Result<SearchUsersResponse, Errors> {
    let response = service_search_users(&state.meilisearch_client, &payload).await?;
    Ok(response)
}
