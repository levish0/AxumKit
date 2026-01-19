use crate::service::posts::service_get_post;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::posts::{GetPostPath, PostResponse};
use axumkit_dto::validator::path_validator::ValidatedPath;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/posts/{id}",
    params(GetPostPath),
    responses(
        (status = 200, description = "Post retrieved successfully", body = PostResponse),
        (status = 404, description = "Post not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Posts"
)]
pub async fn get_post(
    State(state): State<AppState>,
    ValidatedPath(path): ValidatedPath<GetPostPath>,
) -> Result<PostResponse, Errors> {
    service_get_post(&state.conn, &state.seaweedfs_client, path.id).await
}
