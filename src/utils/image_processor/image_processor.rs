use crate::errors::errors::Errors;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, codecs::gif::GifDecoder};
use std::io::Cursor;

pub struct ImageProcessor;

impl ImageProcessor {
    pub fn compress_and_convert(
        data: &[u8],
        output_format: ImageFormat,
        quality: Option<u8>,
        max_dimensions: Option<(u32, u32)>,
    ) -> Result<Vec<u8>, Errors> {
        let img = Self::load_image(data)?;

        let processed_img = if let Some((max_width, max_height)) = max_dimensions {
            Self::resize_if_needed(img, max_width, max_height)
        } else {
            img
        };

        Self::encode_image(processed_img, output_format, quality)
    }

    pub fn is_gif(data: &[u8]) -> bool {
        data.len() >= 3 && &data[0..3] == b"GIF"
    }

    pub fn optimize_gif(data: &[u8]) -> Result<Vec<u8>, Errors> {
        // For GIF files, we'll just validate and return as-is for now
        // Advanced GIF optimization would require more complex processing
        if !Self::is_gif(data) {
            return Err(Errors::BadRequestError("Not a valid GIF file".to_string()));
        }

        // Basic validation by attempting to decode
        let cursor = Cursor::new(data);
        let _decoder = GifDecoder::new(cursor)
            .map_err(|_| Errors::BadRequestError("Invalid GIF format".to_string()))?;

        Ok(data.to_vec())
    }

    fn load_image(data: &[u8]) -> Result<DynamicImage, Errors> {
        let cursor = Cursor::new(data);
        let reader = ImageReader::new(cursor)
            .with_guessed_format()
            .map_err(|_| Errors::BadRequestError("Cannot determine image format".to_string()))?;

        reader
            .decode()
            .map_err(|_| Errors::BadRequestError("Failed to decode image".to_string()))
    }

    fn resize_if_needed(img: DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
        let (width, height) = img.dimensions();

        if width <= max_width && height <= max_height {
            tracing::info!(
                "Image resize: No resize needed - {}x{} fits within {}x{}",
                width,
                height,
                max_width,
                max_height
            );
            return img;
        }

        let ratio = (max_width as f32 / width as f32).min(max_height as f32 / height as f32);
        let new_width = (width as f32 * ratio) as u32;
        let new_height = (height as f32 * ratio) as u32;

        tracing::info!(
            "Image resize: Resizing {}x{} to {}x{} (max: {}x{}, ratio: {:.3})",
            width,
            height,
            new_width,
            new_height,
            max_width,
            max_height,
            ratio
        );

        img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
    }

    fn encode_image(
        img: DynamicImage,
        format: ImageFormat,
        quality: Option<u8>,
    ) -> Result<Vec<u8>, Errors> {
        let mut output = Vec::new();
        let cursor = Cursor::new(&mut output);

        match format {
            ImageFormat::WebP => {
                let encoder = image::codecs::webp::WebPEncoder::new_lossless(cursor);
                img.write_with_encoder(encoder).map_err(|_| {
                    Errors::SysInternalError("Failed to encode WebP image".to_string())
                })?;
            }
            ImageFormat::Jpeg => {
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    cursor,
                    quality.unwrap_or(90),
                );
                encoder.encode_image(&img).map_err(|_| {
                    Errors::SysInternalError("Failed to encode JPEG image".to_string())
                })?;
            }
            ImageFormat::Png => {
                let encoder = image::codecs::png::PngEncoder::new(cursor);
                img.write_with_encoder(encoder).map_err(|_| {
                    Errors::SysInternalError("Failed to encode PNG image".to_string())
                })?;
            }
            _ => {
                return Err(Errors::BadRequestError(
                    "Unsupported output format".to_string(),
                ));
            }
        }

        Ok(output)
    }
}