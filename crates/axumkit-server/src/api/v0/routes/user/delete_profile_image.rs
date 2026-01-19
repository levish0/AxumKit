use crate::extractors::RequiredSession;
use crate::service::user::delete_profile_image::service_delete_profile_image;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    delete,
    path = "/v0/user/me/profile-image",
    responses(
        (status = 204, description = "Profile image deleted successfully"),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 404, description = "Not Found - No profile image to delete"),
        (status = 500, description = "Internal Server Error")
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
    service_delete_profile_image(&state.conn, &state.r2_client, &session_context).await?;
    Ok(StatusCode::NO_CONTENT)
}
