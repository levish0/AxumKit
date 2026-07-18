use crate::extractors::RequiredSession;
use crate::service::user::profile::delete_profile_image::service_delete_profile_image;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    delete,
    path = "/v0/user/me/profile-image",
    summary = "Delete my profile image",
    description = "Deletes the current authenticated user profile image.",
    responses(
        (status = 204, description = "Profile image deleted successfully"),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 404, description = "Not Found - No profile image to delete", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or storage error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User",
)]
pub async fn delete_profile_image(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<StatusCode, Errors> {
    service_delete_profile_image(&state.db, &state.r2_assets, &state.worker, &session_context)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
