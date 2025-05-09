use crate::dto::user_dto::{CreateUserRequest, UserInfoResponse};
use crate::service::error::errors::Errors;
use crate::service::user::user::{service_create_user, service_get_user};
use crate::service::validator::json_validator::ValidatedJson;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    Json, Router,
    routing::{get, post},
};
use tracing::info;

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/user/{id}", get(get_user))
        .route("/user", post(create_user))
}

// GET /user/{id}
async fn get_user(
    state: State<AppState>,
    Path(id): Path<String>,
) -> Result<UserInfoResponse, Errors> {
    info!("Received GET request for user with ID: {}", id);

    let user = service_get_user(&state.conn, id.parse().unwrap()).await?;

    Ok(user)
}

// POST /user
async fn create_user(
    state: State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserRequest>,
) -> Result<impl IntoResponse, Errors> {
    info!("Received POST request to create user: {:?}", payload);

    service_create_user(&state.conn, payload).await?;

    Ok(StatusCode::NO_CONTENT)
}
