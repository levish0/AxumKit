//! Black-box end-to-end test harness for AxumKit.
//!
//! Tests talk to a running stack over HTTP. Bring the stack up with
//! `docker-compose.test.yml` or run `just e2e`.

use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;

pub fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

pub fn mailpit_url() -> String {
    std::env::var("MAILPIT_URL").unwrap_or_else(|_| "http://localhost:8025".to_string())
}

pub fn unique() -> String {
    Uuid::new_v4().simple().to_string()
}

#[derive(Clone)]
pub struct TestClient {
    http: Client,
    base: String,
}

impl Default for TestClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TestClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            base: base_url(),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    pub async fn get(&self, path: &str) -> Response {
        self.http
            .get(self.url(path))
            .send()
            .await
            .expect("GET request failed")
    }

    pub async fn post_json(&self, path: &str, body: &Value) -> Response {
        self.http
            .post(self.url(path))
            .header("X-Turnstile-Token", "e2e-test-token")
            .json(body)
            .send()
            .await
            .expect("POST request failed")
    }

    pub async fn json_ok(resp: Response, expected: StatusCode) -> Value {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        assert_eq!(status, expected, "unexpected status; body: {body}");
        serde_json::from_str(&body).expect("response body should be JSON")
    }

    pub async fn signup_and_verify(&self) -> SignedUpUser {
        let suffix = unique();
        let handle = format!("u{}", &suffix[..12]);
        let email = format!("{handle}@test.invalid");
        let password = "e2e-pass-123";

        let resp = self
            .post_json(
                "/v0/auth/signup",
                &json!({
                    "email": email,
                    "handle": handle,
                    "display_name": "E2E Test User",
                    "password": password,
                }),
            )
            .await;

        assert_eq!(
            resp.status(),
            StatusCode::ACCEPTED,
            "signup should return 202; body: {}",
            resp.text().await.unwrap_or_default()
        );

        let token = wait_for_verification_token(&email).await;
        let resp = self
            .post_json("/v0/auth/verify-email", &json!({ "token": token }))
            .await;

        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "verify-email should return 204; body: {}",
            resp.text().await.unwrap_or_default()
        );

        SignedUpUser {
            handle,
            email,
            password: password.to_string(),
        }
    }
}

pub struct SignedUpUser {
    pub handle: String,
    pub email: String,
    pub password: String,
}

pub async fn wait_for_verification_token(email: &str) -> String {
    let body = wait_for_latest_message_body(email).await;
    extract_token(&body)
        .unwrap_or_else(|| panic!("no `token=` found in verification email to {email}"))
}

pub async fn wait_for_latest_message_body(email: &str) -> String {
    let http = Client::new();
    let search = format!("{}/api/v1/search?query=to:{}", mailpit_url(), email);

    for _ in 0..120 {
        if let Ok(resp) = http.get(&search).send().await
            && resp.status().is_success()
            && let Ok(list) = resp.json::<Value>().await
            && let Some(id) = list["messages"]
                .as_array()
                .and_then(|m| m.first())
                .and_then(|m| m["ID"].as_str())
        {
            let detail_url = format!("{}/api/v1/message/{}", mailpit_url(), id);
            if let Ok(detail) = http.get(&detail_url).send().await
                && let Ok(msg) = detail.json::<Value>().await
            {
                let html = msg["HTML"].as_str().unwrap_or_default();
                let text = msg["Text"].as_str().unwrap_or_default();
                return format!("{text}\n{html}");
            }
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    panic!("timed out waiting for verification email to {email} in mailpit");
}

pub fn extract_token(body: &str) -> Option<String> {
    let start = body.find("token=")? + "token=".len();
    let rest = &body[start..];
    let end = rest
        .find(|c: char| c == '"' || c == '&' || c == '<' || c == '\\' || c.is_whitespace())
        .unwrap_or(rest.len());
    let token = &rest[..end];
    (!token.is_empty()).then(|| token.to_string())
}

#[cfg(test)]
mod tests {
    use super::extract_token;

    #[test]
    fn extracts_token_from_html_href() {
        let body = r#"<a href="http://localhost:5173/account/verify-email?token=abc123_DEF-456">Verify</a>"#;
        assert_eq!(extract_token(body).as_deref(), Some("abc123_DEF-456"));
    }

    #[test]
    fn returns_none_without_token() {
        assert_eq!(extract_token("no token here"), None);
    }
}
