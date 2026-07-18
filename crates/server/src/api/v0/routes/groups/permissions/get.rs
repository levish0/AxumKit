use crate::extractors::RequiredSession;
use crate::service::groups::service_get_group_permissions;
use crate::state::AppState;
use axum::extract::{Query, State};
use dto::groups::GroupPermissionsResponse;
use errors::errors::{ErrorResponse, Errors};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetGroupPermissionsQuery {
    /// Target group id
    pub group_id: Uuid,
}

#[utoipa::path(
    get,
    path = "/v0/groups/permissions",
    summary = "Get a group's permissions",
    description = "Lists the permission codenames granted to a group. Mod or above.",
    params(GetGroupPermissionsQuery),
    responses(
        (status = 200, description = "Group permissions", body = GroupPermissionsResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - Group not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn get_group_permissions(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    Query(query): Query<GetGroupPermissionsQuery>,
) -> Result<GroupPermissionsResponse, Errors> {
    service_get_group_permissions(&state.db, query.group_id, &session).await
}
