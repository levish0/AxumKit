use crate::service::user::account::check_handle_available::service_check_handle_available;
use crate::state::AppState;
use axum::extract::State;
use dto::user::{CheckHandleAvailablePath, CheckHandleAvailableResponse};
use dto::validator::path_validator::ValidatedPath;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/users/handle/{handle}/available",
    summary = "Check handle availability",
    description = "Checks whether the requested user handle is currently available for use.",
    params(CheckHandleAvailablePath),
    responses(
        (status = 200, description = "Handle availability checked", body = CheckHandleAvailableResponse),
        (status = 400, description = "Bad request - Invalid handle format", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "User"
)]
pub async fn check_handle_available(
    State(state): State<AppState>,
    ValidatedPath(path): ValidatedPath<CheckHandleAvailablePath>,
) -> Result<CheckHandleAvailableResponse, Errors> {
    service_check_handle_available(&state.db, &path.handle).await
}
