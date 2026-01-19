use crate::service::search::service_search_posts;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::search::{SearchPostsRequest, SearchPostsResponse};
use axumkit_dto::validator::query_validator::ValidatedQuery;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/search/posts",
    params(SearchPostsRequest),
    responses(
        (status = 200, description = "Posts found successfully", body = SearchPostsResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error"),
        (status = 500, description = "Internal Server Error - MeiliSearch query failed")
    ),
    tag = "Search"
)]
pub async fn search_posts(
    State(state): State<AppState>,
    ValidatedQuery(payload): ValidatedQuery<SearchPostsRequest>,
) -> Result<SearchPostsResponse, Errors> {
    let result = service_search_posts(&state.meilisearch_client, &payload).await?;

    Ok(result)
}
