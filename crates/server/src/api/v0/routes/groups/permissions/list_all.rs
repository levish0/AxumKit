use crate::extractors::RequiredSession;
use crate::service::groups::service_list_permissions;
use crate::state::AppState;
use axum::extract::State;
use dto::groups::PermissionListResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/permissions",
    summary = "List all defined permissions",
    description = "Lists every permission codename the application defines. Mod or above.",
    responses(
        (status = 200, description = "Permission list", body = PermissionListResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn list_permissions(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
) -> Result<PermissionListResponse, Errors> {
    service_list_permissions(&state.db, &session).await
}
