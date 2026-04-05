use crate::extractors::RequiredSession;
use crate::service::user::management::ban_user::service_ban_user;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::user::request::BanUserRequest;
use axumkit_dto::user::response::BanUserResponse;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/users/ban",
    summary = "Ban a user",
    description = "Bans the requested user account and returns the resulting moderation record.",
    request_body = BanUserRequest,
    responses(
        (status = 200, description = "User banned successfully", body = BanUserResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 409, description = "Conflict - User already banned", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User Management"
)]
pub async fn ban_user(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<BanUserRequest>,
) -> Result<BanUserResponse, Errors> {
    service_ban_user(
        &state.db,
        payload.user_id,
        payload.expires_at,
        payload.reason,
        &session,
    )
    .await
}
