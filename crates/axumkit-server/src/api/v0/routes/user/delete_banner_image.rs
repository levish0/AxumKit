use crate::extractors::RequiredSession;
use crate::service::user::delete_banner_image::service_delete_banner_image;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    delete,
    path = "/v0/user/me/banner-image",
    responses(
        (status = 204, description = "Banner image deleted successfully"),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 404, description = "Not Found - No banner image to delete"),
        (status = 500, description = "Internal Server Error")
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User",
)]
pub async fn delete_banner_image(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<StatusCode, Errors> {
    service_delete_banner_image(&state.conn, &state.r2_client, &session_context).await?;
    Ok(StatusCode::NO_CONTENT)
}
