use crate::extractors::RequiredSession;
use crate::service::user::management::unban_user::service_unban_user;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::user::request::UnbanUserRequest;
use axumkit_dto::user::response::UnbanUserResponse;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/users/unban",
    summary = "Unban a user",
    description = "Removes the active ban from the requested user account.",
    request_body = UnbanUserRequest,
    responses(
        (status = 200, description = "User unbanned successfully", body = UnbanUserResponse),
        (status = 400, description = "Bad request - User is not banned", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User Management"
)]
pub async fn unban_user(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<UnbanUserRequest>,
) -> Result<UnbanUserResponse, Errors> {
    service_unban_user(&state.write_db, payload.user_id, payload.reason, &session).await
}
