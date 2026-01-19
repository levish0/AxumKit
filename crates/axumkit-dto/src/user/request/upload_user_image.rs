use crate::validator::multipart_validator::FromMultipart;
use axum::extract::Multipart;
use axumkit_errors::Errors;
use tracing::error;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Validate, ToSchema)]
pub struct UploadUserImageRequest {
    /// Image file binary data
    #[schema(value_type = String, format = Binary, content_media_type = "image/*")]
    pub file: Vec<u8>,
}

impl FromMultipart for UploadUserImageRequest {
    async fn from_multipart(mut multipart: Multipart) -> Result<Self, Errors> {
        let mut file_data = None;

        while let Some(field) = multipart.next_field().await.map_err(|e| {
            error!("Failed to read multipart field: {}", e);
            Errors::BadRequestError("Invalid multipart data".to_string())
        })? {
            let field_name = field.name().map(|s| s.to_string());

            if let Some("file") = field_name.as_deref() {
                let data = field.bytes().await.map_err(|e| {
                    error!("Failed to read file data: {}", e);
                    Errors::FileReadError("Failed to read file data".to_string())
                })?;
                file_data = Some(data.to_vec());
            }
        }

        let file_data = file_data
            .ok_or_else(|| Errors::BadRequestError("Missing required field: file".to_string()))?;

        Ok(Self { file: file_data })
    }
}
