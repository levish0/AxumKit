use anyhow::{Context, anyhow};
use config::WorkerConfig;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use std::sync::LazyLock;
use std::time::Duration;

static MEDIA_PROCESSOR_URL: LazyLock<String> = LazyLock::new(|| {
    WorkerConfig::get()
        .media_processor_url
        .trim_end_matches('/')
        .to_string()
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
) -> Result<ProcessedImage, anyhow::Error> {
    let config = WorkerConfig::get();
    let part = reqwest::multipart::Part::bytes(file).file_name("upload");
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = http_client
        .post(format!("{}/process", &*MEDIA_PROCESSOR_URL))
        .timeout(Duration::from_secs(config.media_processor_timeout_secs))
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                anyhow!("media processor timed out")
            } else {
                anyhow!("media processor request failed: {e}")
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = parse_error_body(response).await;
        return Err(anyhow!("media processor rejected file: {status} {body}"));
    }

    let headers = response.headers().clone();
    let bytes = response
        .bytes()
        .await
        .context("failed to read media processor response")?;

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

fn header_string(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<String, anyhow::Error> {
    headers
        .get(name)
        .ok_or_else(|| anyhow!("missing media processor header: {name}"))?
        .to_str()
        .map(|value| value.to_string())
        .with_context(|| format!("invalid media processor header: {name}"))
}
