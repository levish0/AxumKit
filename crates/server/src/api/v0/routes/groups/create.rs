use crate::extractors::RequiredSession;
use crate::service::groups::service_create_group;
use crate::state::AppState;
use axum::extract::State;
use dto::groups::{CreateGroupRequest, GroupResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/groups",
    summary = "Create an ACL group",
    description = "Creates a new (non-system) ACL group. Admin only.",
    request_body = CreateGroupRequest,
    responses(
        (status = 200, description = "ACL group created successfully", body = GroupResponse),
        (status = 400, description = "Bad request - Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 409, description = "Conflict - Group name already exists", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn create_group(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<CreateGroupRequest>,
) -> Result<GroupResponse, Errors> {
    service_create_group(&state.db, payload, &session).await
}
