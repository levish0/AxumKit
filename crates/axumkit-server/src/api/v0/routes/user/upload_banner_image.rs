use crate::extractors::RequiredSession;
use crate::service::user::upload_banner_image::service_upload_banner_image;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::user::UploadUserImageRequest;
use axumkit_dto::user::UploadUserImageResponse;
use axumkit_dto::validator::multipart_validator::ValidatedMultipart;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/user/me/banner-image",
    request_body(content = UploadUserImageRequest, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Banner image uploaded successfully", body = UploadUserImageResponse),
        (status = 400, description = "Bad Request - Invalid image or validation error"),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 413, description = "Payload Too Large - Image exceeds size limit"),
        (status = 500, description = "Internal Server Error")
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User",
)]
pub async fn upload_banner_image(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedMultipart(payload): ValidatedMultipart<UploadUserImageRequest>,
) -> Result<UploadUserImageResponse, Errors> {
    service_upload_banner_image(&state.conn, &state.r2_client, &session_context, payload).await
}
