use crate::extractors::RequiredSession;
use crate::service::user::management::revoke_role::service_revoke_role;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::user::request::RevokeRoleRequest;
use axumkit_dto::user::response::RevokeRoleResponse;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/users/roles/revoke",
    summary = "Revoke a user role",
    description = "Removes the requested role from the target user account.",
    request_body = RevokeRoleRequest,
    responses(
        (status = 200, description = "Role revoked successfully", body = RevokeRoleResponse),
        (status = 400, description = "Bad request - User does not have this role", body = ErrorResponse),
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
pub async fn revoke_role(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<RevokeRoleRequest>,
) -> Result<RevokeRoleResponse, Errors> {
    service_revoke_role(
        &state.db,
        payload.user_id,
        payload.role,
        payload.reason,
        &session,
    )
    .await
}
