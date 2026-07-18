use crate::connection::r2_assets_conn::R2AssetsClient;
use crate::repository::user::{UserUpdateParams, repository_update_user};
use crate::service::auth::session_types::SessionContext;
use crate::service::user::profile::upload_profile_image::prepare_user_image;
use constants::{BANNER_IMAGE_MAX_SIZE, user_image_key};
use dto::user::UploadUserImageRequest;
use dto::user::UploadUserImageResponse;
use errors::errors::Errors;
use reqwest::Client as HttpClient;
use sea_orm::DatabaseConnection;
use tracing::{info, warn};

/// Uploads my banner image.
///
/// # Responsibilities
/// - Validates the image and strips its metadata.
/// - Uploads to R2, then stores the user's banner image key in the DB.
///
/// # Related
/// - `media_processor_client::process_media`
/// - `R2AssetsClient::upload_with_content_type`
/// - `repository_update_user`
///
/// # Errors
/// - Returns an error on image validation failure or storage failure.
pub async fn service_upload_banner_image(
    db: &DatabaseConnection,
    http_client: &HttpClient,
    r2_assets: &R2AssetsClient,
    session: &SessionContext,
    payload: UploadUserImageRequest,
) -> Result<UploadUserImageResponse, Errors> {
    let prepared_file =
        prepare_user_image(http_client, payload.file, BANNER_IMAGE_MAX_SIZE).await?;

    let storage_key = user_image_key(&prepared_file.hash, &prepared_file.extension);

    r2_assets
        .upload_with_content_type(&storage_key, prepared_file.bytes, &prepared_file.mime_type)
        .await
        .map_err(|e| {
            warn!(user_id = %session.user_id, error = ?e, "Failed to upload banner image to R2");
            Errors::SysInternalError("Failed to upload image to storage".to_string())
        })?;

    repository_update_user(
        db,
        session.user_id,
        UserUpdateParams {
            banner_image: Some(Some(storage_key.clone())),
            ..Default::default()
        },
    )
    .await?;

    let image_url = r2_assets.get_public_url(&storage_key);

    info!(user_id = %session.user_id, storage_key = %storage_key, "Banner image uploaded");

    Ok(UploadUserImageResponse { image_url })
}
