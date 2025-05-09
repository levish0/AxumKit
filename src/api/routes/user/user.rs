use crate::dto::user_dto::CreateUser;
use crate::service::error::errors::Errors;
use crate::service::user::user::create_user;
use crate::service::validator::json_validator::ValidatedJson;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    Router,
    routing::{get, post},
};
use tracing::info;

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/user/{id}", get(get_user_handler))
        .route("/user", post(create_user_handler))
}

// GET /user/{id}
async fn get_user_handler(Path(id): Path<String>) -> String {
    info!("Received GET request for user with ID: {}", id);
    format!("User with ID: {}", id)
}

// POST /user
async fn create_user_handler(
    state: State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUser>,
) -> Result<(impl IntoResponse), Errors> {
    create_user(&state.conn, payload).await?;

    Ok(StatusCode::NO_CONTENT)
}
