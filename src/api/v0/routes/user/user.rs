use crate::dto::user_dto::{CreateUserRequest, UserInfoResponse};
use crate::service::error::errors::Errors;
use crate::service::user::user::{service_create_user, service_get_user};
use crate::service::validator::json_validator::ValidatedJson;
use crate::state::AppState;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use tracing::info;

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/user/{id}", get(get_user))
        .route("/user", post(create_user))
}

#[utoipa::path(
    get,
    path = "/v0/user/{id}",
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = StatusCode::OK, description = "Successfully retrieved user information", body = UserInfoResponse),
        (status = StatusCode::NOT_FOUND, description = "User not found"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error")
    ),
    tag = "User"
)]
pub async fn get_user(
    state: State<AppState>,
    Path(id): Path<i32>,
) -> Result<UserInfoResponse, Errors> {
    info!("Received GET request for user with ID: {}", id);

    let user = service_get_user(&state.conn, id).await?;
    Ok(user)
}

// POST /user
#[utoipa::path(
    post,
    path = "/v0/user",
    request_body = CreateUserRequest,
    responses(
        (status = StatusCode::NO_CONTENT, description = "User created successfully"),
        (status = StatusCode::BAD_REQUEST, description = "Invalid request"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error")
    ),
    tag = "User"
)]
pub async fn create_user(
    state: State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserRequest>,
) -> Result<impl IntoResponse, Errors> {
    info!("Received POST request to create user: {:?}", payload);

    service_create_user(&state.conn, payload).await?;

    Ok(StatusCode::NO_CONTENT)
}
