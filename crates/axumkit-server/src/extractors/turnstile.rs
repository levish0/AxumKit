use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;

use crate::bridge::turnstile_client::verify_turnstile_token;
use crate::state::AppState;
use axumkit_config::ServerConfig;
use axumkit_errors::errors::Errors;

/// Turnstile 헤더 이름
pub const TURNSTILE_TOKEN_HEADER: &str = "X-Turnstile-Token";

/// Cloudflare Turnstile 검증 완료를 나타내는 extractor
///
/// 핸들러에 이 extractor를 추가하면 Turnstile 토큰 검증이 자동으로 수행됩니다.
/// 검증 실패 시 요청이 거부되고 핸들러 본문은 실행되지 않습니다.
///
/// # 사용 예시
/// ```rust,ignore
/// pub async fn create_document(
///     State(state): State<AppState>,
///     _turnstile: TurnstileVerified,  // 이 줄 추가하면 검증됨
///     Json(req): Json<CreateDocumentRequest>,
/// ) -> Result<impl IntoResponse, Errors> {
///     // Turnstile 검증 통과한 요청만 여기 도달
/// }
/// ```
///
/// # 클라이언트 사용법
/// ```typescript
/// const response = await fetch('/api/v0/document', {
///   method: 'POST',
///   headers: {
///     'Content-Type': 'application/json',
///     'X-Turnstile-Token': turnstileToken,
///   },
///   body: JSON.stringify(data),
/// });
/// ```
#[derive(Debug, Clone)]
pub struct TurnstileVerified;

impl<S> FromRequestParts<S> for TurnstileVerified
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = Errors;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        // 1. 헤더에서 토큰 추출
        let token = parts
            .headers
            .get(TURNSTILE_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or(Errors::TurnstileTokenMissing)?;

        // 2. 클라이언트 IP 추출 (Cloudflare 환경에서는 CF-Connecting-IP 사용)
        let remote_ip = parts
            .headers
            .get("CF-Connecting-IP")
            .and_then(|v| v.to_str().ok());

        // 3. Cloudflare API로 검증
        let config = ServerConfig::get();
        let response = verify_turnstile_token(
            &app_state.http_client,
            &config.turnstile_secret_key,
            token,
            remote_ip,
        )
        .await?;

        // 4. 검증 결과 확인
        if !response.success {
            return Err(Errors::TurnstileVerificationFailed);
        }

        Ok(TurnstileVerified)
    }
}
