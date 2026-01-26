use crate::extractors::RequiredSession;
use crate::service::user::get_my_profile::service_get_my_profile;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::user::UserResponse;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/user/me",
    responses(
        (status = 200, description = "Current user info", body = UserResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 500, description = "Internal Server Error - Database error")
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
    service_get_my_profile(&state.write_db, &session_context).await
}
