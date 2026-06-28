use config::ServerConfig;
use errors::errors::Errors;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::error;

static MEDIA_PROCESSOR_URL: LazyLock<String> = LazyLock::new(|| {
    let config = ServerConfig::get();
    config.media_processor_url.trim_end_matches('/').to_string()
});

pub struct ProcessedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub extension: String,
}

#[derive(Debug, Deserialize)]
struct MediaProcessorErrorBody {
    code: Option<String>,
    error: Option<String>,
}

pub async fn process_media(
    http_client: &HttpClient,
    file: Vec<u8>,
) -> Result<ProcessedImage, Errors> {
    let config = ServerConfig::get();
    let part = reqwest::multipart::Part::bytes(file).file_name("upload");
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = http_client
        .post(format!("{}/process", &*MEDIA_PROCESSOR_URL))
        .timeout(Duration::from_secs(config.media_processor_timeout_secs))
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            error!("Media processor request failed: {e}");
            if e.is_timeout() {
                Errors::FileProcessingTimeout("Image processing timed out".to_string())
            } else {
                Errors::FileProcessingUnavailable("Media processor is unavailable".to_string())
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = parse_error_body(response).await;
        return Err(map_processor_error(status, body));
    }

    let headers = response.headers().clone();
    let bytes = response.bytes().await.map_err(|e| {
        error!("Failed to read media processor response: {e}");
        Errors::SysInternalError("Invalid media processor response".to_string())
    })?;

    Ok(ProcessedImage {
        bytes: bytes.to_vec(),
        mime_type: header_string(&headers, "x-image-mime-type")?,
        extension: header_string(&headers, "x-image-extension")?,
    })
}

async fn parse_error_body(response: reqwest::Response) -> String {
    let fallback = response.text().await.unwrap_or_default();
    serde_json::from_str::<MediaProcessorErrorBody>(&fallback)
        .ok()
        .and_then(|body| body.error.or(body.code))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback)
}

fn map_processor_error(status: reqwest::StatusCode, body: String) -> Errors {
    let details = if body.trim().is_empty() {
        format!("Media processor returned HTTP {}", status.as_u16())
    } else {
        body
    };

    match status.as_u16() {
        408 => Errors::FileProcessingTimeout(details),
        413 => Errors::FileTooLargeError(details),
        415 => Errors::FileUnsupportedType(details),
        400..=499 => Errors::BadRequestError(details),
        _ => {
            error!(status = %status, body = %details, "Media processor request failed");
            Errors::FileProcessingUnavailable(
                "Media processor failed to process the file".to_string(),
            )
        }
    }
}

fn header_string(headers: &reqwest::header::HeaderMap, name: &str) -> Result<String, Errors> {
    headers
        .get(name)
        .ok_or_else(|| Errors::SysInternalError(format!("Missing media processor header: {name}")))?
        .to_str()
        .map(|value| value.to_string())
        .map_err(|_| Errors::SysInternalError(format!("Invalid media processor header: {name}")))
}
