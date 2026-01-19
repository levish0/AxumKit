use crate::connection::r2_conn::R2Client;
use crate::repository::user::{UserUpdateParams, repository_update_user};
use crate::service::auth::session_types::SessionContext;
use crate::utils::image_processor::image_validator::{
    MAX_IMAGE_SIZE, generate_image_hash, process_image_for_upload, validate_image,
};
use axumkit_constants::user_image_key;
use axumkit_dto::user::UploadUserImageRequest;
use axumkit_dto::user::UploadUserImageResponse;
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;
use tracing::{error, info};

pub async fn service_upload_profile_image(
    conn: &DatabaseConnection,
    r2_client: &R2Client,
    session: &SessionContext,
    payload: UploadUserImageRequest,
) -> Result<UploadUserImageResponse, Errors> {
    let image_info = validate_image(&payload.file, MAX_IMAGE_SIZE)?;

    info!(
        "Uploading profile image: user_id={}, mime_type={}, size={} bytes",
        session.user_id, image_info.mime_type, image_info.size
    );

    let processed = process_image_for_upload(&payload.file, &image_info.mime_type)?;

    let hash = generate_image_hash(&processed.data);
    let storage_key = user_image_key(&hash, &processed.extension);

    r2_client
        .upload_with_content_type(&storage_key, processed.data, &processed.content_type)
        .await
        .map_err(|e| {
            error!("Failed to upload profile image to R2: {}", e);
            Errors::SysInternalError("Failed to upload image to storage".to_string())
        })?;

    repository_update_user(
        conn,
        session.user_id,
        UserUpdateParams {
            profile_image: Some(Some(storage_key.clone())),
            ..Default::default()
        },
    )
    .await?;

    let image_url = r2_client.get_r2_public_url(&storage_key);

    info!("Profile image uploaded successfully: {}", image_url);

    Ok(UploadUserImageResponse { image_url })
}
