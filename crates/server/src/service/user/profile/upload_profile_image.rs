use crate::bridge::media_processor_client::process_media;
use crate::connection::r2_assets_conn::R2AssetsClient;
use crate::repository::user::{UserUpdateParams, repository_update_user};
use crate::service::auth::session_types::SessionContext;
use crate::service::user::utils::spawn_index_user;
use crate::state::WorkerClient;
use constants::{PROFILE_IMAGE_MAX_SIZE, user_image_key};
use dto::user::UploadUserImageRequest;
use dto::user::UploadUserImageResponse;
use errors::errors::Errors;
use reqwest::Client as HttpClient;
use sea_orm::DatabaseConnection;
use tracing::{info, warn};

/// Uploads my profile image.
///
/// # Responsibilities
/// - Validates the image and strips its metadata.
/// - Uploads to R2, then stores the user's profile image key in the DB.
/// - Triggers a search index refresh.
///
/// # Related
/// - `media_processor_client::process_media`
/// - `R2AssetsClient::upload_with_content_type`
/// - `repository_update_user`
/// - `worker_client::index_user`
///
/// # Errors
/// - Returns an error on image validation failure or storage failure.
pub async fn service_upload_profile_image(
    db: &DatabaseConnection,
    http_client: &HttpClient,
    r2_assets: &R2AssetsClient,
    worker: &WorkerClient,
    session: &SessionContext,
    payload: UploadUserImageRequest,
) -> Result<UploadUserImageResponse, Errors> {
    let prepared_file =
        prepare_user_image(http_client, payload.file, PROFILE_IMAGE_MAX_SIZE).await?;

    let storage_key = user_image_key(&prepared_file.hash, &prepared_file.extension);

    r2_assets
        .upload_with_content_type(&storage_key, prepared_file.bytes, &prepared_file.mime_type)
        .await
        .map_err(|e| {
            warn!(user_id = %session.user_id, error = ?e, "Failed to upload profile image to R2");
            Errors::SysInternalError("Failed to upload image to storage".to_string())
        })?;

    repository_update_user(
        db,
        session.user_id,
        UserUpdateParams {
            profile_image: Some(Some(storage_key.clone())),
            ..Default::default()
        },
    )
    .await?;

    let image_url = r2_assets.get_public_url(&storage_key);

    // Index the user in MeiliSearch (reflect profile image change)
    spawn_index_user(worker, session.user_id);

    info!(user_id = %session.user_id, storage_key = %storage_key, "Profile image uploaded");

    Ok(UploadUserImageResponse { image_url })
}

pub(super) struct PreparedUserImage {
    pub(super) bytes: Vec<u8>,
    pub(super) mime_type: String,
    pub(super) extension: String,
    pub(super) hash: String,
}

pub(super) async fn prepare_user_image(
    http_client: &HttpClient,
    file: Vec<u8>,
    max_size: usize,
) -> Result<PreparedUserImage, Errors> {
    if file.is_empty() {
        return Err(Errors::BadRequestError("Empty file".to_string()));
    }

    if file.len() > max_size {
        return Err(Errors::FileTooLargeError(format!(
            "File too large: {} bytes (max: {} bytes)",
            file.len(),
            max_size
        )));
    }

    let processed = process_media(http_client, file).await?;
    let bytes = processed.bytes;

    if bytes.is_empty() {
        return Err(Errors::BadRequestError(
            "Processed image is empty".to_string(),
        ));
    }

    if bytes.len() > max_size {
        return Err(Errors::FileTooLargeError(format!(
            "Processed file too large: {} bytes (max: {} bytes)",
            bytes.len(),
            max_size
        )));
    }

    let mime_type = processed.mime_type;
    if mime_type != "image/webp" {
        return Err(Errors::BadRequestError(format!(
            "Unexpected processed image type: {mime_type}"
        )));
    }

    let extension = processed.extension;
    if extension != "webp" {
        return Err(Errors::BadRequestError(format!(
            "Unexpected processed image extension: {extension}"
        )));
    }

    let hash = blake3::hash(&bytes).to_hex().to_string();

    Ok(PreparedUserImage {
        bytes,
        mime_type,
        extension,
        hash,
    })
}
