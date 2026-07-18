use crate::extractors::RequiredSession;
use crate::service::groups::service_delete_group;
use crate::state::AppState;
use axum::extract::State;
use dto::groups::{DeleteGroupRequest, GroupResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/groups/delete",
    summary = "Delete an ACL group",
    description = "Deletes a non-system ACL group that no rules reference. Admin only.",
    request_body = DeleteGroupRequest,
    responses(
        (status = 200, description = "ACL group deleted successfully", body = GroupResponse),
        (status = 400, description = "Bad request - System group or still referenced by rules", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - Group not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn delete_group(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<DeleteGroupRequest>,
) -> Result<GroupResponse, Errors> {
    service_delete_group(&state.db, payload, &session).await
}
