use crate::extractors::session::RequiredSession;
use crate::service::posts::service_delete_post;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::posts::{DeletePostResponse, GetPostPath};
use axumkit_dto::validator::path_validator::ValidatedPath;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    delete,
    path = "/v0/posts/{id}",
    params(GetPostPath),
    responses(
        (status = 200, description = "Post deleted successfully", body = DeletePostResponse),
        (status = 401, description = "Unauthorized - User not authenticated or not the author"),
        (status = 404, description = "Post not found"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Posts"
)]
pub async fn delete_post(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedPath(path): ValidatedPath<GetPostPath>,
) -> Result<DeletePostResponse, Errors> {
    service_delete_post(&state.write_db, &state.worker, path.id, session.user_id).await
}
