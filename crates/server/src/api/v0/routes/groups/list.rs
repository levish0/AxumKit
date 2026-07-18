use crate::extractors::RequiredSession;
use crate::service::groups::service_list_groups;
use crate::state::AppState;
use axum::extract::State;
use dto::groups::GroupListResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/groups",
    summary = "List ACL groups",
    description = "Returns every ACL group. Requires the document moderator role.",
    responses(
        (status = 200, description = "ACL groups retrieved successfully", body = GroupListResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn list_groups(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
) -> Result<GroupListResponse, Errors> {
    service_list_groups(&state.db, &session).await
}
