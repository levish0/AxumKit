use axumkit_errors::errors::Errors;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

const TURNSTILE_VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/// Cloudflare Turnstile 검증 응답
#[derive(Debug, Deserialize)]
pub struct TurnstileResponse {
    /// 검증 성공 여부
    pub success: bool,
    /// 에러 코드 목록 (실패 시)
    #[serde(rename = "error-codes", default)]
    pub error_codes: Vec<String>,
    /// 챌린지 완료 시간 (ISO 8601)
    #[serde(default)]
    pub challenge_ts: Option<String>,
    /// 챌린지가 표시된 호스트
    #[serde(default)]
    pub hostname: Option<String>,
    /// 클라이언트에서 전달한 action
    #[serde(default)]
    pub action: Option<String>,
    /// 클라이언트에서 전달한 cdata
    #[serde(default)]
    pub cdata: Option<String>,
}

/// Cloudflare Turnstile API 요청
#[derive(Debug, Serialize)]
struct TurnstileRequest<'a> {
    secret: &'a str,
    response: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    remoteip: Option<&'a str>,
}

/// Cloudflare Turnstile 토큰 검증
///
/// # Arguments
/// * `http_client` - HTTP 클라이언트
/// * `secret_key` - Turnstile 시크릿 키
/// * `token` - 클라이언트에서 받은 토큰
/// * `remote_ip` - 클라이언트 IP (선택)
///
/// # Returns
/// * `Ok(TurnstileResponse)` - 검증 응답 (success 필드 확인 필요)
/// * `Err(Errors::TurnstileServiceError)` - API 호출 실패
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
