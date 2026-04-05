use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;

use crate::bridge::turnstile_client::verify_turnstile_token;
use crate::state::AppState;
use axumkit_config::ServerConfig;
use axumkit_errors::errors::Errors;

/// Turnstile header name
pub const TURNSTILE_TOKEN_HEADER: &str = "X-Turnstile-Token";

/// Extractor indicating Cloudflare Turnstile verification is complete
///
/// Adding this extractor to a handler automatically performs Turnstile token verification.
/// On verification failure, the request is rejected and the handler body is not executed.
///
/// # Usage Example
/// ```rust,ignore
/// pub async fn create_document(
///     State(state): State<AppState>,
///     _turnstile: TurnstileVerified,  // Adding this line enables verification
///     Json(req): Json<CreateDocumentRequest>,
/// ) -> Result<impl IntoResponse, Errors> {
///     // Only requests that passed Turnstile verification reach here
/// }
/// ```
///
/// # Client Usage
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

        // 1. Extract token from header
        let token = parts
            .headers
            .get(TURNSTILE_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or(Errors::TurnstileTokenMissing)?;

        // 2. Extract client IP (uses CF-Connecting-IP in Cloudflare environments)
        let remote_ip = parts
            .headers
            .get("CF-Connecting-IP")
            .and_then(|v| v.to_str().ok());

        // 3. Verify via Cloudflare API
        let config = ServerConfig::get();
        let response = verify_turnstile_token(
            &app_state.http_client,
            &config.turnstile_secret_key,
            token,
            remote_ip,
        )
        .await?;

        // 4. Check verification result
        if !response.success {
            return Err(Errors::TurnstileVerificationFailed);
        }

        Ok(TurnstileVerified)
    }
}
