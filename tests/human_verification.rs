mod support;

#[path = "human_verification/api.rs"]
mod api;

use anyhow::Result;
use application::{
    ApplicationError,
    config::SecretString,
    human_verification::{HumanVerificationAction, HumanVerificationService, TurnstileSettings},
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::{Value, json};
use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;

const SITE_HOSTNAME: &str = "app.example.test";

/// A local siteverify replacement. The submitted token selects the scripted answer.
struct FakeSiteverify {
    state: Arc<FakeSiteverifyState>,
    url: url::Url,
    task: JoinHandle<()>,
}
struct FakeSiteverifyState {
    // Generated at runtime so tests keep credentials out of source literals.
    secret: String,
    requests: Mutex<Vec<Value>>,
}

impl FakeSiteverify {
    async fn start() -> Result<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}/turnstile/v0/siteverify", listener.local_addr()?).parse()?;
        let state = Arc::new(FakeSiteverifyState {
            secret: auth_core::token::generate_secure_token(),
            requests: Mutex::new(Vec::new()),
        });
        let router = Router::new()
            .route("/turnstile/v0/siteverify", post(siteverify))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Ok(Self { state, url, task })
    }

    fn settings(&self) -> TurnstileSettings {
        TurnstileSettings {
            secret_key: SecretString::new(self.state.secret.clone()),
            siteverify_url: self.url.clone(),
            expected_hostnames: vec![SITE_HOSTNAME.into()],
            verify_action: true,
            attempt_timeout: Duration::from_millis(500),
        }
    }

    fn service(&self, settings: TurnstileSettings) -> Result<HumanVerificationService> {
        Ok(HumanVerificationService::new(Some(settings))?)
    }

    fn secret(&self) -> &str {
        &self.state.secret
    }

    fn take_requests(&self) -> Vec<Value> {
        std::mem::take(&mut *self.state.requests.lock().unwrap())
    }
}

impl Drop for FakeSiteverify {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn answer(success: bool, hostname: &str, action: &str, error_codes: &[&str]) -> Response {
    Json(json!({
        "success": success,
        "challenge_ts": "2026-10-17T00:00:00.000Z",
        "hostname": hostname,
        "action": action,
        "cdata": "",
        "error-codes": error_codes,
        "metadata": {"interactive": false}
    }))
    .into_response()
}

async fn siteverify(
    State(state): State<Arc<FakeSiteverifyState>>,
    Json(request): Json<Value>,
) -> Response {
    let previous_attempts = {
        let mut requests = state.requests.lock().unwrap();
        let previous_attempts = requests
            .iter()
            .filter(|previous| previous["idempotency_key"] == request["idempotency_key"])
            .count();
        requests.push(request.clone());
        previous_attempts
    };
    if request["secret"] != state.secret.as_str() {
        return answer(false, "", "", &["invalid-input-secret"]);
    }
    let token = request["response"].as_str().unwrap_or_default();
    if let Some(action) = token.strip_prefix("valid:") {
        return answer(true, SITE_HOSTNAME, action, &[]);
    }
    if let Some(action) = token.strip_prefix("retried:") {
        if previous_attempts == 0 {
            return StatusCode::BAD_GATEWAY.into_response();
        }
        return answer(true, SITE_HOSTNAME, action, &[]);
    }
    match token {
        "other-host" => answer(true, "attacker.example.test", "signup", &[]),
        "rejected" => answer(false, "", "", &["invalid-input-response"]),
        "spent" => answer(false, "", "", &["timeout-or-duplicate"]),
        "unknown-code" => answer(false, "", "", &["new-provider-code"]),
        "internal-error" => answer(false, "", "", &["internal-error"]),
        "unavailable" => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        "malformed" => "not json".into_response(),
        "slow" => {
            tokio::time::sleep(Duration::from_secs(3)).await;
            answer(true, SITE_HOSTNAME, "signup", &[])
        }
        _ => answer(false, "", "", &["invalid-input-response"]),
    }
}

fn is_failed(result: Result<(), ApplicationError>) -> bool {
    matches!(result, Err(ApplicationError::HumanVerificationFailed))
}

fn is_unavailable(result: Result<(), ApplicationError>) -> bool {
    matches!(result, Err(ApplicationError::HumanVerificationUnavailable))
}

#[tokio::test]
async fn unconfigured_verification_accepts_requests_without_a_provider() -> Result<()> {
    let service = HumanVerificationService::new(None)?;
    for token in [None, Some(""), Some("any token")] {
        service
            .verify(HumanVerificationAction::Signup, token, None)
            .await?;
    }
    Ok(())
}

#[tokio::test]
async fn accepted_tokens_send_secret_token_client_address_and_idempotency_key() -> Result<()> {
    let provider = FakeSiteverify::start().await?;
    let service = provider.service(provider.settings())?;
    let client: IpAddr = "203.0.113.7".parse()?;
    service
        .verify(
            HumanVerificationAction::Signup,
            Some("valid:signup"),
            Some(client),
        )
        .await?;
    service
        .verify(HumanVerificationAction::Login, Some("valid:login"), None)
        .await?;
    let requests = provider.take_requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["secret"], provider.secret());
    assert_eq!(requests[0]["response"], "valid:signup");
    assert_eq!(requests[0]["remoteip"], "203.0.113.7");
    assert!(requests[1].get("remoteip").is_none());
    let keys: Vec<uuid::Uuid> = requests
        .iter()
        .map(|request| request["idempotency_key"].as_str().unwrap().parse())
        .collect::<Result<_, _>>()?;
    assert_ne!(keys[0], keys[1]);
    Ok(())
}

#[tokio::test]
async fn missing_rejected_and_reused_tokens_fail_verification() -> Result<()> {
    let provider = FakeSiteverify::start().await?;
    let service = provider.service(provider.settings())?;
    for token in [None, Some("")] {
        assert!(is_failed(
            service
                .verify(HumanVerificationAction::Signup, token, None)
                .await
        ));
    }
    assert!(matches!(
        service
            .verify(
                HumanVerificationAction::Signup,
                Some(&"x".repeat(2049)),
                None
            )
            .await,
        Err(ApplicationError::InvalidInput(_))
    ));
    assert!(provider.take_requests().is_empty());
    for token in ["rejected", "spent", "unknown-code"] {
        assert!(is_failed(
            service
                .verify(HumanVerificationAction::Signup, Some(token), None)
                .await
        ));
    }
    assert_eq!(provider.take_requests().len(), 3);
    Ok(())
}

#[tokio::test]
async fn hostname_and_action_mismatches_fail_only_when_configured() -> Result<()> {
    let provider = FakeSiteverify::start().await?;
    let strict = provider.service(provider.settings())?;
    assert!(is_failed(
        strict
            .verify(HumanVerificationAction::Signup, Some("other-host"), None)
            .await
    ));
    assert!(is_failed(
        strict
            .verify(HumanVerificationAction::Signup, Some("valid:login"), None)
            .await
    ));
    assert!(is_failed(
        strict
            .verify(HumanVerificationAction::PasswordReset, Some("valid:"), None)
            .await
    ));
    let mut settings = provider.settings();
    settings.expected_hostnames.clear();
    settings.verify_action = false;
    let lenient = provider.service(settings)?;
    for token in ["other-host", "valid:login", "valid:"] {
        lenient
            .verify(HumanVerificationAction::Signup, Some(token), None)
            .await?;
    }
    Ok(())
}

#[tokio::test]
async fn provider_failures_are_unavailable_and_retries_reuse_the_idempotency_key() -> Result<()> {
    let provider = FakeSiteverify::start().await?;
    let service = provider.service(provider.settings())?;
    service
        .verify(
            HumanVerificationAction::Signup,
            Some("retried:signup"),
            None,
        )
        .await?;
    let requests = provider.take_requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0]["idempotency_key"],
        requests[1]["idempotency_key"]
    );

    for token in ["unavailable", "internal-error"] {
        assert!(is_unavailable(
            service
                .verify(HumanVerificationAction::Signup, Some(token), None)
                .await
        ));
        let requests = provider.take_requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0]["idempotency_key"],
            requests[1]["idempotency_key"]
        );
    }
    assert!(is_unavailable(
        service
            .verify(HumanVerificationAction::Signup, Some("malformed"), None)
            .await
    ));
    assert_eq!(provider.take_requests().len(), 1);

    let started = std::time::Instant::now();
    assert!(is_unavailable(
        service
            .verify(HumanVerificationAction::Signup, Some("slow"), None)
            .await
    ));
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(provider.take_requests().len(), 2);

    let mut settings = provider.settings();
    settings.secret_key = SecretString::new(auth_core::token::generate_secure_token());
    let misconfigured = provider.service(settings)?;
    assert!(is_unavailable(
        misconfigured
            .verify(HumanVerificationAction::Signup, Some("valid:signup"), None)
            .await
    ));
    assert_eq!(provider.take_requests().len(), 1);

    let closed_port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    let mut settings = provider.settings();
    settings.siteverify_url =
        format!("http://127.0.0.1:{closed_port}/turnstile/v0/siteverify").parse()?;
    let unreachable = provider.service(settings)?;
    assert!(is_unavailable(
        unreachable
            .verify(HumanVerificationAction::Signup, Some("valid:signup"), None)
            .await
    ));
    Ok(())
}
