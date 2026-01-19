use crate::utils::image_processor::image_processor::ImageProcessor;
use axumkit_errors::errors::Errors;
use blake3;
use image::{ImageFormat, ImageReader};
use infer;
use std::io::Cursor;

/// Maximum image size before optimization (10MB)
pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

/// Allowed image MIME types
const ALLOWED_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

/// Image information after validation
#[derive(Debug)]
pub struct ImageInfo {
    pub mime_type: String,
    pub extension: String,
    pub size: usize,
    pub hash: String,
    pub width: u32,
    pub height: u32,
}

/// Validate image and return image information
pub fn validate_image(data: &[u8], max_size: usize) -> Result<ImageInfo, Errors> {
    if data.is_empty() {
        return Err(Errors::BadRequestError("Empty file".to_string()));
    }

    // Check file size
    if data.len() > max_size {
        return Err(Errors::FileTooLargeError(format!(
            "Image too large: {} bytes (max: {} bytes)",
            data.len(),
            max_size
        )));
    }

    // Detect file type using infer crate
    let kind = infer::get(data).ok_or_else(|| {
        Errors::BadRequestError("Cannot determine file type or unsupported format".to_string())
    })?;

    let mime_type = kind.mime_type();

    // Check if file type is an allowed image type
    if !ALLOWED_IMAGE_TYPES.contains(&mime_type) {
        return Err(Errors::BadRequestError(format!(
            "Unsupported image type: {}. Allowed types: JPEG, PNG, GIF, WebP",
            mime_type
        )));
    }

    let extension = kind.extension();

    // Generate hash for filename
    let hash = generate_image_hash(data);

    // Get image dimensions
    let (width, height) = get_image_dimensions(data)?;

    Ok(ImageInfo {
        mime_type: mime_type.to_string(),
        extension: extension.to_string(),
        size: data.len(),
        hash,
        width,
        height,
    })
}

/// Get image dimensions from raw data (efficient - reads header only, no full decode)
fn get_image_dimensions(data: &[u8]) -> Result<(u32, u32), Errors> {
    let cursor = Cursor::new(data);
    let reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|_| Errors::BadRequestError("Cannot determine image format".to_string()))?;

    reader
        .into_dimensions()
        .map_err(|_| Errors::BadRequestError("Failed to read image dimensions".to_string()))
}

/// Processed image result
#[derive(Debug)]
pub struct ProcessedImage {
    pub data: Vec<u8>,
    pub content_type: String,
    pub extension: String,
    pub width: u32,
    pub height: u32,
}

/// Process image for upload (compression and optimization)
pub fn process_image_for_upload(data: &[u8], mime_type: &str) -> Result<ProcessedImage, Errors> {
    // Validate it's an allowed image type
    if !ALLOWED_IMAGE_TYPES.contains(&mime_type) {
        return Err(Errors::BadRequestError(
            "File is not a supported image type".to_string(),
        ));
    }

    // Handle GIF files separately - keep them as GIF
    if mime_type == "image/gif" {
        let optimized_data = ImageProcessor::optimize_gif(data)?;
        let (width, height) = get_image_dimensions(&optimized_data)?;
        return Ok(ProcessedImage {
            data: optimized_data,
            content_type: "image/gif".to_string(),
            extension: "gif".to_string(),
            width,
            height,
        });
    }

    // For other image types, compress and convert to WebP
    let max_dimensions = Some((2000, 2000)); // Max dimensions for wiki images
    let (compressed_data, width, height) = ImageProcessor::compress_and_convert_with_dimensions(
        data,
        ImageFormat::WebP,
        Some(90), // Quality 90%
        max_dimensions,
    )?;

    Ok(ProcessedImage {
        data: compressed_data,
        content_type: "image/webp".to_string(),
        extension: "webp".to_string(),
        width,
        height,
    })
}

/// Generate blake3 hash for image content
pub fn generate_image_hash(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    hash.to_hex().to_string()
}
