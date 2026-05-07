use errors::errors::Errors;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

const TURNSTILE_VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/// Cloudflare Turnstile verification response
#[derive(Debug, Deserialize)]
pub struct TurnstileResponse {
    /// Whether verification succeeded
    pub success: bool,
    /// Error code list (on failure)
    #[serde(rename = "error-codes", default)]
    pub error_codes: Vec<String>,
    /// Challenge completion time (ISO 8601)
    #[serde(default)]
    pub challenge_ts: Option<String>,
    /// Host where the challenge was displayed
    #[serde(default)]
    pub hostname: Option<String>,
    /// Action passed from the client
    #[serde(default)]
    pub action: Option<String>,
    /// cdata passed from the client
    #[serde(default)]
    pub cdata: Option<String>,
}

/// Cloudflare Turnstile API request
#[derive(Debug, Serialize)]
struct TurnstileRequest<'a> {
    secret: &'a str,
    response: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    remoteip: Option<&'a str>,
}

/// Cloudflare Turnstile token verification
///
/// # Arguments
/// * `http_client` - HTTP client
/// * `secret_key` - Turnstile secret key
/// * `token` - Token received from the client
/// * `remote_ip` - Client IP (optional)
///
/// # Returns
/// * `Ok(TurnstileResponse)` - Verification response (check the success field)
/// * `Err(Errors::TurnstileServiceError)` - API call failed
pub async fn verify_turnstile_token(
    http_client: &HttpClient,
    secret_key: &str,
    token: &str,
    remote_ip: Option<&str>,
) -> Result<TurnstileResponse, Errors> {
    let request_body = TurnstileRequest {
        secret: secret_key,
        response: token,
        remoteip: remote_ip,
    };

    let response = http_client
        .post(TURNSTILE_VERIFY_URL)
        .json(&request_body)
        .send()
        .await
        .map_err(|_| Errors::TurnstileServiceError)?;

    if !response.status().is_success() {
        return Err(Errors::TurnstileServiceError);
    }

    response
        .json::<TurnstileResponse>()
        .await
        .map_err(|_| Errors::TurnstileServiceError)
}
