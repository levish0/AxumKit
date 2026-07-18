use crate::extractors::RequiredSession;
use crate::service::acl::service_get_acl_group_permissions;
use crate::state::AppState;
use axum::extract::{Query, State};
use dto::acl::AclGroupPermissionsResponse;
use errors::errors::{ErrorResponse, Errors};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetAclGroupPermissionsQuery {
    /// Target group id
    pub group_id: Uuid,
}

#[utoipa::path(
    get,
    path = "/v0/acl/groups/permissions",
    summary = "Get a group's permissions",
    description = "Lists the permission codenames granted to a group. Mod or above.",
    params(GetAclGroupPermissionsQuery),
    responses(
        (status = 200, description = "Group permissions", body = AclGroupPermissionsResponse),
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
pub async fn get_acl_group_permissions(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    Query(query): Query<GetAclGroupPermissionsQuery>,
) -> Result<AclGroupPermissionsResponse, Errors> {
    service_get_acl_group_permissions(&state.db, query.group_id, &session).await
}
