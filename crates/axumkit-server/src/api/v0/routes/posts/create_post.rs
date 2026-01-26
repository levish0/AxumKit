use crate::extractors::session::RequiredSession;
use crate::service::posts::service_create_post;
use crate::state::AppState;
use axum::{extract::State, response::IntoResponse};
use axumkit_dto::posts::{CreatePostRequest, CreatePostResponse};
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/posts",
    request_body = CreatePostRequest,
    responses(
        (status = 201, description = "Post created successfully", body = CreatePostResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error"),
        (status = 401, description = "Unauthorized - User not authenticated"),
        (status = 500, description = "Internal Server Error"),
    ),
    tag = "Posts"
)]
pub async fn create_post(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<CreatePostRequest>,
) -> Result<impl IntoResponse, Errors> {
    let response = service_create_post(
        &state.write_db,
        &state.seaweedfs_client,
        &state.worker,
        session.user_id,
        payload.title,
        payload.content,
    )
    .await?;

    Ok(response)
}
