use axum::extract::{ConnectInfo, FromRef, FromRequestParts};
use axum::http::request::Parts;
use std::net::SocketAddr;

use crate::bridge::turnstile_client::verify_turnstile_token;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use config::ServerConfig;
use errors::errors::Errors;

/// Turnstile header name
pub const TURNSTILE_TOKEN_HEADER: &str = "X-Turnstile-Token";

/// Extractor indicating Cloudflare Turnstile verification is complete
///
/// Adding this extractor to a handler runs Turnstile token verification automatically.
/// On verification failure the request is rejected and the handler body never runs.
///
/// # Usage example
/// ```rust,ignore
/// pub async fn create_document(
///     State(state): State<AppState>,
///     _turnstile: TurnstileVerified,  // adding this line enables verification
///     Json(req): Json<CreateDocumentRequest>,
/// ) -> Result<impl IntoResponse, Errors> {
///     // only requests that passed Turnstile verification reach here
/// }
/// ```
///
/// # Client usage
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

        // 1. Extract the token from the header
        let token = parts
            .headers
            .get(TURNSTILE_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or(Errors::TurnstileTokenMissing)?;

        // 2. Extract the client IP (CF-Connecting-IP behind Cloudflare)
        let remote_ip = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|connect_info| extract_ip_address(&parts.headers, connect_info.0));

        // 3. Verify against the Cloudflare API
        let config = ServerConfig::get();
        let response = verify_turnstile_token(
            &app_state.http_client,
            &config.turnstile_verify_url,
            &config.turnstile_secret_key,
            token,
            remote_ip.as_deref(),
        )
        .await?;

        // 4. Check the verification result
        if !response.success {
            return Err(Errors::TurnstileVerificationFailed);
        }

        Ok(TurnstileVerified)
    }
}
