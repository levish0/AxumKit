use crate::extractors::RequiredSession;
use crate::service::user::management::grant_role::service_grant_role;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::user::request::GrantRoleRequest;
use axumkit_dto::user::response::GrantRoleResponse;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/users/roles/grant",
    summary = "Grant a user role",
    description = "Grants the requested role to the target user account.",
    request_body = GrantRoleRequest,
    responses(
        (status = 200, description = "Role granted successfully", body = GrantRoleResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 409, description = "Conflict - User already has this role", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User Management"
)]
pub async fn grant_role(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<GrantRoleRequest>,
) -> Result<GrantRoleResponse, Errors> {
    service_grant_role(
        &state.db,
        payload.user_id,
        payload.role,
        payload.expires_at,
        payload.reason,
        &session,
    )
    .await
}
