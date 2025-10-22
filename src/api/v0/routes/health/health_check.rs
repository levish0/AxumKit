use crate::errors::errors::Errors;
use axum::http::StatusCode;
use axum::response::IntoResponse;

#[utoipa::path(
    get,
    path = "/v0/health_check",
    responses(
        (status = 204, description = "Service is healthy and running"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Health"
)]
pub async fn health_check() -> Result<impl IntoResponse, Errors> {
    Ok(StatusCode::NO_CONTENT)
}
