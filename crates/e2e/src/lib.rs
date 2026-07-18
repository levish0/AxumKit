//! Black-box end-to-end test harness for the AxumKit API.
//!
//! Tests drive a fully running stack (server + worker + Postgres + Redis + NATS +
//! MeiliSearch + SeaweedFS + mailpit, brought up by `docker-compose.test.yml`) purely
//! over HTTP. Nothing here imports the server crate, so the harness mirrors a real client.
//!
//! Configuration (env vars, with defaults matching the compose port mappings):
//!   - `E2E_BASE_URL`  API base URL                (default `http://127.0.0.1:18000` —
//!     127.0.0.1 rather than `localhost`: on hosts where `localhost` resolves to ::1
//!     first, Docker's IPv6 port proxy can time out instead of connecting)
//!   - `MAILPIT_URL`   mailpit web/REST API base   (default `http://127.0.0.1:18025`)

use std::time::Duration;

use entity::common::Role;
use reqwest::{Client, Response, StatusCode};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
    QueryFilter, Set,
};
use serde_json::{Value, json};
use uuid::Uuid;

/// API base URL under test.
pub fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:18000".to_string())
}

/// mailpit REST API base URL.
pub fn mailpit_url() -> String {
    std::env::var("MAILPIT_URL").unwrap_or_else(|_| "http://127.0.0.1:18025".to_string())
}

/// Connection string for the disposable test database.
///
/// Default port 55432 matches `docker-compose.test.yml`, which deliberately avoids host
/// port 5432: a natively installed Postgres (or the dev stack) listening on
/// 0.0.0.0:5432 silently shadows the container mapping and fails auth.
pub fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://axumkit:axumkit@127.0.0.1:55432/axumkit".to_string())
}

/// Opens a short-lived, single-connection pool for one out-of-band DB access.
/// Callers must `close()` it before returning.
///
/// Deliberately NOT a shared `static` pool: every `#[tokio::test]` runs on its own
/// tokio runtime, and sqlx connections are bound to the IO driver of the runtime
/// that created them. A pooled connection created by an earlier test gets polled
/// from a later test's runtime and hangs forever (observed on Windows), surfacing
/// as `ConnectionAcquire(Timeout)` in `grant_role`. One connection per call, closed
/// deterministically, is runtime-safe and still keeps the harness footprint at
/// ≤ test-threads connections — far below the pile-up of default-sized (~10 conn)
/// never-closed pools that once exhausted Postgres `max_connections` under
/// parallel `cargo test`.
async fn test_db() -> DatabaseConnection {
    let mut opt = ConnectOptions::new(database_url());
    opt.max_connections(1).min_connections(0);
    Database::connect(opt)
        .await
        .expect("connect to test database")
}

/// Grants an explicit role to a signed-up user by inserting straight into `user_roles`.
///
/// Bootstrap rationale: the app exposes no first-admin path (roles are granted only by an
/// existing admin), so privileged-policy e2e tests elevate a fresh user out-of-band against
/// the disposable test database. The server reads roles per request, so the new role takes
/// effect on the user's next call without re-login.
pub async fn grant_role(handle: &str, role: Role) {
    let db = test_db().await;
    let user = entity::users::Entity::find()
        .filter(entity::users::Column::Handle.eq(handle))
        .one(&db)
        .await
        .expect("query user by handle")
        .unwrap_or_else(|| panic!("no user with handle {handle}"));

    entity::user_roles::ActiveModel {
        user_id: Set(user.id),
        role: Set(role),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("insert user_role");

    db.close().await.expect("close test db connection");
}

/// Rewinds a user's `created_at` by `days` via the disposable test database.
///
/// The ACL `user_age_days` condition compares against account age computed at
/// request time, and freshly signed-up e2e users are always 0 days old — so
/// exercising the "old enough" branch requires aging the account out-of-band
/// (same bootstrap rationale as [`grant_role`]).
pub async fn backdate_user(handle: &str, days: i64) {
    let db = test_db().await;
    let user = entity::users::Entity::find()
        .filter(entity::users::Column::Handle.eq(handle))
        .one(&db)
        .await
        .expect("query user by handle")
        .unwrap_or_else(|| panic!("no user with handle {handle}"));

    let mut active: entity::users::ActiveModel = user.into();
    active.created_at = Set(chrono::Utc::now() - chrono::Duration::days(days));
    active.update(&db).await.expect("backdate user created_at");

    db.close().await.expect("close test db connection");
}

/// Returns a unique suffix so concurrent tests never collide on handle/email.
///
/// Uses a v4 (fully random) UUID — NOT v7, whose leading hex is a millisecond timestamp
/// shared by tests that start in the same millisecond under parallel `cargo test`.
pub fn unique() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Encodes a 4x4 solid-color RGBA PNG in memory. Distinct colors produce distinct
/// bytes (and therefore distinct blake3 storage keys), which the file-replace flows
/// rely on to register as an actual change.
pub fn test_png(rgba: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 4, 4);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        let pixels: Vec<u8> = rgba.iter().copied().cycle().take(4 * 4 * 4).collect();
        writer.write_image_data(&pixels).expect("png data");
    }
    out
}

/// A cookie-aware HTTP client bound to the API base URL. A fresh client has its own
/// cookie jar, so each one represents an independent browser/session.
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
    /// A new anonymous client (empty cookie jar).
    pub fn new() -> Self {
        Self::build(None)
    }

    /// A client whose requests present `ip` as the client address via the
    /// `CF-Connecting-IP` header. The server runs behind Cloudflare Tunnel and trusts
    /// that header, so tests can express per-actor IPs (e.g. for IP-ban policy tests)
    /// without real network differences. Use TEST-NET addresses (203.0.113.0/24).
    pub fn with_ip(ip: &str) -> Self {
        Self::build(Some(ip))
    }

    fn build(ip: Option<&str>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ip) = ip {
            headers.insert("CF-Connecting-IP", ip.parse().expect("valid IP header"));
        }
        let http = Client::builder()
            .cookie_store(true)
            .default_headers(headers)
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

    /// GET with URL-encoded query pairs (for titles containing spaces etc.).
    pub async fn get_q(&self, path: &str, query: &[(&str, &str)]) -> Response {
        self.http
            .get(self.url(path))
            .query(query)
            .send()
            .await
            .expect("GET request failed")
    }

    /// POST JSON. A Turnstile token header is always attached; the test stack uses
    /// Cloudflare's always-passes test secret, so any value verifies.
    pub async fn post_json(&self, path: &str, body: &Value) -> Response {
        self.http
            .post(self.url(path))
            .header("X-Turnstile-Token", "e2e-test-token")
            .json(body)
            .send()
            .await
            .expect("POST request failed")
    }

    /// POST JSON with URL-encoded query pairs (e.g. `/v0/document/edit?namespace=..&title=..`).
    pub async fn post_json_q(&self, path: &str, query: &[(&str, &str)], body: &Value) -> Response {
        self.http
            .post(self.url(path))
            .query(query)
            .header("X-Turnstile-Token", "e2e-test-token")
            .json(body)
            .send()
            .await
            .expect("POST request failed")
    }

    /// POST a multipart form (file-document endpoints).
    pub async fn post_multipart(&self, path: &str, form: reqwest::multipart::Form) -> Response {
        self.http
            .post(self.url(path))
            .header("X-Turnstile-Token", "e2e-test-token")
            .multipart(form)
            .send()
            .await
            .expect("multipart POST failed")
    }

    /// PATCH JSON (profile updates use PATCH).
    pub async fn patch_json(&self, path: &str, body: &Value) -> Response {
        self.http
            .patch(self.url(path))
            .header("X-Turnstile-Token", "e2e-test-token")
            .json(body)
            .send()
            .await
            .expect("PATCH request failed")
    }

    /// PUT JSON (user preferences use PUT).
    pub async fn put_json(&self, path: &str, body: &Value) -> Response {
        self.http
            .put(self.url(path))
            .header("X-Turnstile-Token", "e2e-test-token")
            .json(body)
            .send()
            .await
            .expect("PUT request failed")
    }

    /// DELETE (session revocation, preference removal).
    pub async fn delete(&self, path: &str) -> Response {
        self.http
            .delete(self.url(path))
            .header("X-Turnstile-Token", "e2e-test-token")
            .send()
            .await
            .expect("DELETE request failed")
    }

    /// DELETE with a JSON body (e.g. re-authentication payloads).
    pub async fn delete_json(&self, path: &str, body: &Value) -> Response {
        self.http
            .delete(self.url(path))
            .header("X-Turnstile-Token", "e2e-test-token")
            .json(body)
            .send()
            .await
            .expect("DELETE request failed")
    }

    /// Fetches the authenticated user's own profile (`GET /v0/user/me`), asserting 200.
    pub async fn me(&self) -> Value {
        let resp = self.get("/v0/user/me").await;
        Self::json_ok(resp, StatusCode::OK).await
    }

    /// Parses a successful JSON response body, asserting the expected status first.
    pub async fn json_ok(resp: Response, expected: StatusCode) -> Value {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        assert_eq!(status, expected, "unexpected status; body: {body}");
        serde_json::from_str(&body).expect("response body should be JSON")
    }

    /// Signs up a brand new user, consumes the verification email from mailpit, and
    /// completes verification — which sets the session cookie, leaving this client
    /// logged in. Returns the created user's (handle, email).
    pub async fn signup_and_login(&self) -> SignedUpUser {
        let suffix = unique();
        let handle = format!("u{}", &suffix[..12]);
        let email = format!("{handle}@test.invalid");
        let password = "e2e-password-123";

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
            "verify-email should return 204 and set the session cookie; body: {}",
            resp.text().await.unwrap_or_default()
        );

        SignedUpUser {
            handle,
            email,
            password: password.to_string(),
        }
    }
}

/// Details of a user created via [`TestClient::signup_and_login`].
pub struct SignedUpUser {
    pub handle: String,
    pub email: String,
    pub password: String,
}

/// Polls mailpit for the newest message addressed to `email` and extracts the
/// `?token=...` value from the verification link. Panics after a timeout.
pub async fn wait_for_verification_token(email: &str) -> String {
    let body = wait_for_latest_message_body(email).await;
    extract_token(&body)
        .unwrap_or_else(|| panic!("no `token=` found in verification email to {email}"))
}

/// Returns the id of the newest mailpit message addressed to `email`, if any.
/// Capture this BEFORE triggering a new email, then pass it to
/// [`wait_for_new_message_body`] so the older message (e.g. the signup verification
/// email) is not mistaken for the new one.
pub async fn latest_message_id(email: &str) -> Option<String> {
    let http = Client::new();
    let search = format!("{}/api/v1/search?query=to:{}", mailpit_url(), email);
    let resp = http.get(&search).send().await.ok()?;
    let list = resp.json::<Value>().await.ok()?;
    list["messages"]
        .as_array()
        .and_then(|m| m.first())
        .and_then(|m| m["ID"].as_str())
        .map(str::to_string)
}

/// Polls mailpit until a message NEWER than `previous_id` arrives for `email`, then
/// returns its combined text+HTML body.
pub async fn wait_for_new_message_body(email: &str, previous_id: Option<&str>) -> String {
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
            && Some(id) != previous_id
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

    panic!("timed out waiting for a new email to {email} in mailpit");
}

/// Polls mailpit for the newest message to `email` and returns its combined text+HTML
/// body. ~30s budget: tolerates worker NATS-consumer cold start on the first email.
pub async fn wait_for_latest_message_body(email: &str) -> String {
    wait_for_new_message_body(email, None).await
}

/// Extracts the value after the first `token=` occurrence, stopping at the first
/// character that cannot appear in the URL-safe token (`"`, `&`, `<`, whitespace).
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
        let body =
            r#"<a href="http://localhost:5173/u/verify-email?token=abc123_DEF-456">Verify</a>"#;
        assert_eq!(extract_token(body).as_deref(), Some("abc123_DEF-456"));
    }

    #[test]
    fn returns_none_without_token() {
        assert_eq!(extract_token("no token here"), None);
    }
}
