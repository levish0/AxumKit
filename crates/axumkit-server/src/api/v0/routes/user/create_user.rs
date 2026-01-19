use crate::service::user::create_user::service_create_user;
use crate::state::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axumkit_dto::user::{CreateUserRequest, CreateUserResponse};
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = CreateUserResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error"),
        (status = 409, description = "Conflict - User with this email or handle already exists"),
        (status = 500, description = "Internal Server Error - Database or Redis error"),
        (status = 502, description = "Bad Gateway - Worker service request failed or returned invalid response"),
        (status = 503, description = "Service Unavailable - Worker service connection failed"),
    ),
    tag = "User"
)]
pub async fn create_user(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserRequest>,
) -> Result<impl IntoResponse, Errors> {
    let response =
        service_create_user(&state.conn, &state.redis_session, &state.worker, payload).await?;

    Ok((StatusCode::CREATED, Json(response)))
}
