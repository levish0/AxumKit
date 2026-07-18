use crate::extractors::RequiredSession;
use crate::service::user::profile::get_my_profile::service_get_my_profile;
use crate::state::AppState;
use axum::extract::State;
use dto::user::UserResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/user/me",
    summary = "Get my profile",
    description = "Returns the profile for the current authenticated user.",
    responses(
        (status = 200, description = "Current user info", body = UserResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User",
)]
pub async fn get_my_profile(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<UserResponse, Errors> {
    service_get_my_profile(&state.db, &session_context).await
}
