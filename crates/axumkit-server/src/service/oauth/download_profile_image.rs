use crate::connection::R2Client;
use crate::utils::image_processor::image_validator::{
    generate_image_hash, process_image_for_upload, validate_image,
};
use axumkit_constants::{PROFILE_IMAGE_MAX_SIZE, user_image_key};
use reqwest::Client as HttpClient;
use tracing::{error, warn};

/// OAuth 프로필 이미지를 다운로드하여 R2에 업로드하고 storage key를 반환합니다.
///
/// 실패 시 None을 반환합니다 (프로필 이미지 없이 가입 진행).
pub async fn download_and_upload_profile_image(
    http_client: &HttpClient,
    r2_assets: &R2Client,
    image_url: &str,
) -> Option<String> {
    // 1. 이미지 다운로드
    let response = match http_client.get(image_url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            warn!(error = ?e, "Failed to download OAuth profile image");
            return None;
        }
    };

    if !response.status().is_success() {
        warn!(status = %response.status(), "OAuth profile image download failed");
        return None;
    }

    let image_bytes = match response.bytes().await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => {
            warn!(error = ?e, "Failed to read OAuth profile image bytes");
            return None;
        }
    };

    // 2. 이미지 검증
    let image_info = match validate_image(&image_bytes, PROFILE_IMAGE_MAX_SIZE) {
        Ok(info) => info,
        Err(e) => {
            warn!(error = ?e, "OAuth profile image validation failed");
            return None;
        }
    };

    // 3. 이미지 처리 (WebP 변환, 리사이즈)
    let processed = match process_image_for_upload(&image_bytes, &image_info.mime_type) {
        Ok(p) => p,
        Err(e) => {
            warn!(error = ?e, "OAuth profile image processing failed");
            return None;
        }
    };

    // 4. R2 업로드
    let hash = generate_image_hash(&processed.data);
    let storage_key = user_image_key(&hash, &processed.extension);

    if let Err(e) = r2_assets
        .upload_with_content_type(&storage_key, processed.data, &processed.content_type)
        .await
    {
        error!(error = ?e, "Failed to upload OAuth profile image to R2");
        return None;
    }

    Some(storage_key)
}
