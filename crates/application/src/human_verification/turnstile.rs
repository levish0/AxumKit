use super::{HumanVerificationAction, TurnstileSettings};
use crate::{ApplicationError, ApplicationResult};
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, time::Duration};
use uuid::Uuid;

const MAX_RESPONSE_BYTES: usize = 16_384;

pub(super) struct TurnstileClient {
    client: Client,
    settings: TurnstileSettings,
}

#[derive(Serialize)]
struct SiteverifyRequest<'a> {
    secret: &'a str,
    response: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    remoteip: Option<String>,
    idempotency_key: Uuid,
}

#[derive(Deserialize)]
struct SiteverifyResponse {
    success: bool,
    #[serde(default)]
    hostname: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default, rename = "error-codes")]
    error_codes: Vec<SiteverifyErrorCode>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum SiteverifyErrorCode {
    MissingInputSecret,
    InvalidInputSecret,
    MissingInputResponse,
    InvalidInputResponse,
    BadRequest,
    TimeoutOrDuplicate,
    InternalError,
    // Cloudflare can add codes. An unknown code still means the token did not pass.
    #[serde(other)]
    Unrecognized,
}

enum SiteverifyOutcome {
    Answered(SiteverifyResponse),
    Retryable,
    Unusable,
}

impl TurnstileClient {
    pub(super) fn new(settings: TurnstileSettings) -> ApplicationResult<Self> {
        settings.validate()?;
        let client = Client::builder()
            .timeout(settings.attempt_timeout)
            .connect_timeout(settings.attempt_timeout.min(Duration::from_secs(3)))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(ApplicationError::internal)?;
        Ok(Self { client, settings })
    }

    pub(super) async fn verify(
        &self,
        action: HumanVerificationAction,
        token: &str,
        remote_ip: Option<IpAddr>,
    ) -> ApplicationResult<()> {
        // Cloudflare treats a retry with the same key as the same validation,
        // not as a second use of the single-use token.
        let request = SiteverifyRequest {
            secret: self.settings.secret_key.expose(),
            response: token,
            remoteip: remote_ip.map(|address| address.to_string()),
            idempotency_key: Uuid::new_v4(),
        };
        let mut outcome = self.request_siteverify(&request).await;
        if matches!(outcome, SiteverifyOutcome::Retryable) {
            outcome = self.request_siteverify(&request).await;
        }
        let response = match outcome {
            SiteverifyOutcome::Answered(response) => response,
            SiteverifyOutcome::Retryable | SiteverifyOutcome::Unusable => {
                tracing::warn!(
                    failure = "siteverify_unavailable",
                    "human verification unavailable"
                );
                return Err(ApplicationError::HumanVerificationUnavailable);
            }
        };
        if !response.success {
            if response.error_codes.iter().any(|code| {
                matches!(
                    code,
                    SiteverifyErrorCode::MissingInputSecret
                        | SiteverifyErrorCode::InvalidInputSecret
                        | SiteverifyErrorCode::BadRequest
                )
            }) {
                tracing::error!(
                    failure = "siteverify_configuration_rejected",
                    "human verification unavailable"
                );
                return Err(ApplicationError::HumanVerificationUnavailable);
            }
            return Err(ApplicationError::HumanVerificationFailed);
        }
        if !self.settings.expected_hostnames.is_empty()
            && !response.hostname.as_deref().is_some_and(|hostname| {
                self.settings
                    .expected_hostnames
                    .iter()
                    .any(|expected| expected.eq_ignore_ascii_case(hostname))
            })
        {
            return Err(ApplicationError::HumanVerificationFailed);
        }
        if self.settings.verify_action && response.action.as_deref() != Some(action.as_str()) {
            return Err(ApplicationError::HumanVerificationFailed);
        }
        Ok(())
    }

    async fn request_siteverify(&self, request: &SiteverifyRequest<'_>) -> SiteverifyOutcome {
        let Ok(response) = self
            .client
            .post(self.settings.siteverify_url.clone())
            .json(request)
            .send()
            .await
        else {
            return SiteverifyOutcome::Retryable;
        };
        let status = response.status();
        if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
            return SiteverifyOutcome::Retryable;
        }
        if !status.is_success() {
            return SiteverifyOutcome::Unusable;
        }
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let Ok(chunk) = chunk else {
                return SiteverifyOutcome::Retryable;
            };
            if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return SiteverifyOutcome::Unusable;
            }
            body.extend_from_slice(&chunk);
        }
        match serde_json::from_slice::<SiteverifyResponse>(&body) {
            Ok(answer)
                if !answer.success
                    && answer
                        .error_codes
                        .contains(&SiteverifyErrorCode::InternalError) =>
            {
                SiteverifyOutcome::Retryable
            }
            Ok(answer) => SiteverifyOutcome::Answered(answer),
            Err(_) => SiteverifyOutcome::Unusable,
        }
    }
}
