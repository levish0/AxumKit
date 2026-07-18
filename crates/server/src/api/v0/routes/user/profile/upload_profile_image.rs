use crate::extractors::RequiredSession;
use crate::service::user::profile::upload_profile_image::service_upload_profile_image;
use crate::state::AppState;
use axum::extract::State;
use dto::user::UploadUserImageRequest;
use dto::user::UploadUserImageResponse;
use dto::validator::multipart_validator::ValidatedMultipart;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/user/me/profile-image",
    summary = "Upload my profile image",
    description = "Uploads or replaces the profile image for the current authenticated user profile. The uploaded file must be JPEG, PNG, GIF, or WebP and is sanitized into WebP before storage.",
    request_body(content = UploadUserImageRequest, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Profile image uploaded successfully", body = UploadUserImageResponse),
        (status = 400, description = "Bad Request - Invalid image or validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 413, description = "Payload Too Large - Image exceeds size limit", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or storage error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User",
)]
pub async fn upload_profile_image(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedMultipart(payload): ValidatedMultipart<UploadUserImageRequest>,
) -> Result<UploadUserImageResponse, Errors> {
    service_upload_profile_image(
        &state.db,
        &state.http_client,
        &state.r2_assets,
        &state.worker,
        &session_context,
        payload,
    )
    .await
}
