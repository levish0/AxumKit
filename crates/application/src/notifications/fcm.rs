use crate::{ApplicationError, ApplicationResult};
use futures_util::StreamExt;
use gcp_auth::TokenProvider;
use reqwest::{Client, StatusCode};
use std::{sync::Arc, time::Duration};
use tokio::sync::OnceCell;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct FcmSettings {
    pub project_id: String,
}
impl FcmSettings {
    pub fn validate(&self) -> ApplicationResult<()> {
        let value = self.project_id.as_bytes();
        if !(6..=30).contains(&value.len())
            || !value[0].is_ascii_lowercase()
            || !value.last().is_some_and(u8::is_ascii_alphanumeric)
            || !value
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        {
            return Err(ApplicationError::InvalidInput("invalid FCM project ID"));
        }
        Ok(())
    }
}

pub(super) struct FcmClient {
    client: Client,
    endpoint: String,
    provider: OnceCell<Arc<dyn TokenProvider>>,
}
#[derive(Debug)]
pub(super) struct FcmFailure {
    pub code: &'static str,
    pub retryable: bool,
    pub retry_after_seconds: Option<u64>,
    pub unregistered: bool,
}
impl FcmFailure {
    fn unavailable() -> Self {
        Self {
            code: "fcm_unavailable",
            retryable: true,
            retry_after_seconds: None,
            unregistered: false,
        }
    }
}

impl FcmClient {
    pub(super) fn new(settings: FcmSettings) -> ApplicationResult<Self> {
        settings.validate()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(ApplicationError::internal)?;
        Ok(Self {
            client,
            endpoint: format!(
                "https://fcm.googleapis.com/v1/projects/{}/messages:send",
                settings.project_id
            ),
            provider: OnceCell::new(),
        })
    }

    pub(super) async fn send(
        &self,
        token: &str,
        notification_id: Uuid,
        delivery_id: Uuid,
    ) -> Result<(), FcmFailure> {
        let oauth = tokio::time::timeout(Duration::from_secs(10), async {
            let provider = self.provider.get_or_try_init(gcp_auth::provider).await?;
            provider
                .token(&["https://www.googleapis.com/auth/firebase.messaging"])
                .await
        })
        .await
        .map_err(|_| FcmFailure::unavailable())?
        .map_err(|_| FcmFailure::unavailable())?;
        self.send_authorized(oauth.as_str(), token, notification_id, delivery_id)
            .await
    }

    async fn send_authorized(
        &self,
        oauth: &str,
        device_token: &str,
        notification_id: Uuid,
        delivery_id: Uuid,
    ) -> Result<(), FcmFailure> {
        let response=self.client.post(&self.endpoint).bearer_auth(oauth).json(&serde_json::json!({"message":{
            "token":device_token,
            "notification":{"title":"New notification","body":"Open the app to view this notification."},
            "data":{"notification_id":notification_id.to_string(),"delivery_id":delivery_id.to_string()},
            "android":{"ttl":"3600s","collapse_key":notification_id.to_string()},
            "apns":{"headers":{"apns-collapse-id":notification_id.to_string()}}
        }})).send().await.map_err(|_|FcmFailure::unavailable())?;
        let status = response.status();
        let retry_after_seconds = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| {
                value.parse::<u64>().ok().or_else(|| {
                    chrono::DateTime::parse_from_rfc2822(value)
                        .ok()
                        .map(|date| {
                            (date.with_timezone(&chrono::Utc) - chrono::Utc::now())
                                .num_seconds()
                                .max(1) as u64
                        })
                })
            })
            .map(|seconds| seconds.clamp(1, 86400));
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| FcmFailure::unavailable())?;
            if bytes.len() + chunk.len() > 65536 {
                return Err(FcmFailure::unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        if status.is_success() {
            return Ok(());
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        let unregistered = value
            .pointer("/error/details")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|details| {
                details.iter().any(|detail| {
                    detail.get("@type").and_then(serde_json::Value::as_str)
                        == Some("type.googleapis.com/google.firebase.fcm.v1.FcmError")
                        && detail.get("errorCode").and_then(serde_json::Value::as_str)
                            == Some("UNREGISTERED")
                })
            });
        let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
        Err(FcmFailure {
            code: if unregistered {
                "fcm_unregistered"
            } else if retryable {
                "fcm_retryable"
            } else {
                "fcm_rejected"
            },
            retryable,
            retry_after_seconds: retry_after_seconds
                .or((status == StatusCode::TOO_MANY_REQUESTS).then_some(60)),
            unregistered,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn response(status: &str, body: &str, extra: &str) -> Result<(), FcmFailure> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{body}",
            body.len()
        );
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut chunk = [0; 4096];
                let length = socket.read(&mut chunk).await.unwrap();
                assert!(length > 0 && request.len() + length <= 16384);
                request.extend_from_slice(&chunk[..length]);
                if let Some(split) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..split]).to_lowercase();
                    let content_length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .unwrap()
                        .trim()
                        .parse()
                        .unwrap();
                    if request.len() >= split + 4 + content_length {
                        let payload: serde_json::Value =
                            serde_json::from_slice(&request[split + 4..split + 4 + content_length])
                                .unwrap();
                        assert_eq!(
                            payload["message"]["notification"]["title"],
                            "New notification"
                        );
                        assert!(payload["message"]["data"]["notification_id"].is_string());
                        assert!(payload["message"]["data"]["delivery_id"].is_string());
                        assert!(payload["message"]["data"].get("body").is_none());
                        break;
                    }
                }
            }
            socket.write_all(reply.as_bytes()).await.unwrap();
        });
        let mut client = FcmClient::new(FcmSettings {
            project_id: "demo-project".into(),
        })
        .unwrap();
        client.endpoint = format!("http://{address}");
        let result = client
            .send_authorized(
                "test-only-oauth",
                "test-only-device-token",
                Uuid::now_v7(),
                Uuid::now_v7(),
            )
            .await;
        task.await.unwrap();
        result
    }

    #[tokio::test]
    async fn provider_errors_distinguish_invalid_payload_tokens_and_retry_after() {
        assert!(
            response("200 OK", "{\"name\":\"projects/demo/messages/test\"}", "")
                .await
                .is_ok()
        );
        let invalid = response(
            "400 Bad Request",
            "{\"error\":{\"status\":\"INVALID_ARGUMENT\"}}",
            "",
        )
        .await
        .unwrap_err();
        assert!(!invalid.unregistered && !invalid.retryable);
        let unregistered=response("404 Not Found","{\"error\":{\"details\":[{\"@type\":\"type.googleapis.com/google.firebase.fcm.v1.FcmError\",\"errorCode\":\"UNREGISTERED\"}]}}","").await.unwrap_err();
        assert!(unregistered.unregistered);
        let retry = response("429 Too Many Requests", "{}", "Retry-After: 120\r\n")
            .await
            .unwrap_err();
        assert!(retry.retryable);
        assert_eq!(retry.retry_after_seconds, Some(120));
        assert!(
            response("503 Service Unavailable", "not-json", "")
                .await
                .unwrap_err()
                .retryable
        );
    }
}
