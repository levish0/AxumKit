use crate::extractors::session::RequiredSession;
use crate::service::posts::service_update_post;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::posts::{GetPostPath, PostResponse, UpdatePostRequest};
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_dto::validator::path_validator::ValidatedPath;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    patch,
    path = "/v0/posts/{id}",
    params(GetPostPath),
    request_body = UpdatePostRequest,
    responses(
        (status = 200, description = "Post updated successfully", body = PostResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error"),
        (status = 401, description = "Unauthorized - User not authenticated or not the author"),
        (status = 404, description = "Post not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Posts"
)]
pub async fn update_post(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedPath(path): ValidatedPath<GetPostPath>,
    ValidatedJson(payload): ValidatedJson<UpdatePostRequest>,
) -> Result<PostResponse, Errors> {
    service_update_post(
        &state.conn,
        &state.seaweedfs_client,
        &state.worker,
        path.id,
        session.user_id,
        payload.title,
        payload.content,
    )
    .await
}
