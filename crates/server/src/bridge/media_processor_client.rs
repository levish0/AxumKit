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

pub struct ProcessedMedia {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub extension: String,
    pub width: u64,
    pub height: u64,
    pub animated: bool,
    pub pages: u64,
}

#[derive(Debug, Deserialize)]
struct MediaProcessorErrorBody {
    code: Option<String>,
    error: Option<String>,
}

pub async fn process_media(
    http_client: &HttpClient,
    file: Vec<u8>,
) -> Result<ProcessedMedia, Errors> {
    let config = ServerConfig::get();
    let part = reqwest::multipart::Part::bytes(file).file_name("upload");
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = http_client
        .post(format!("{}/process", *MEDIA_PROCESSOR_URL))
        .timeout(Duration::from_secs(config.media_processor_timeout_secs))
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            error!("Media processor request failed: {e}");
            if e.is_timeout() {
                Errors::FileProcessingTimeout("Media processing timed out".to_string())
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

    Ok(ProcessedMedia {
        bytes: bytes.to_vec(),
        mime_type: header_string(&headers, "x-media-mime-type")?,
        extension: header_string(&headers, "x-media-extension")?,
        width: header_u64(&headers, "x-media-width")?,
        height: header_u64(&headers, "x-media-height")?,
        // `x-media-animated`/`x-media-pages` are image-only extras; non-image
        // outputs omit them and are rejected later by the webp mime check.
        animated: header_bool_or(&headers, "x-media-animated", false)?,
        pages: header_u64_or(&headers, "x-media-pages", 0)?,
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
        .map(|v| v.to_string())
        .map_err(|_| Errors::SysInternalError(format!("Invalid media processor header: {name}")))
}

fn header_u64(headers: &reqwest::header::HeaderMap, name: &str) -> Result<u64, Errors> {
    header_string(headers, name)?
        .parse::<u64>()
        .map_err(|_| Errors::SysInternalError(format!("Invalid media processor header: {name}")))
}

fn header_bool(headers: &reqwest::header::HeaderMap, name: &str) -> Result<bool, Errors> {
    header_string(headers, name)?
        .parse::<bool>()
        .map_err(|_| Errors::SysInternalError(format!("Invalid media processor header: {name}")))
}

fn header_u64_or(
    headers: &reqwest::header::HeaderMap,
    name: &str,
    default: u64,
) -> Result<u64, Errors> {
    match headers.get(name) {
        Some(_) => header_u64(headers, name),
        None => Ok(default),
    }
}

fn header_bool_or(
    headers: &reqwest::header::HeaderMap,
    name: &str,
    default: bool,
) -> Result<bool, Errors> {
    match headers.get(name) {
        Some(_) => header_bool(headers, name),
        None => Ok(default),
    }
}
