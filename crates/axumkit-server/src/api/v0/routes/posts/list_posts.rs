use crate::service::posts::service_list_posts;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use axumkit_dto::posts::{ListPostsQuery, ListPostsResponse};
use axumkit_errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/posts",
    params(
        ("limit" = u64, Query, description = "Number of posts to return"),
        ("offset" = u64, Query, description = "Number of posts to skip")
    ),
    responses(
        (status = 200, description = "Posts retrieved successfully", body = ListPostsResponse),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Posts"
)]
pub async fn list_posts(
    State(state): State<AppState>,
    Query(query): Query<ListPostsQuery>,
) -> Result<impl IntoResponse, Errors> {
    let response = service_list_posts(&state.read_db, query.limit, query.offset).await?;
    Ok(response)
}
