use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::google::service_issue_google_one_tap_nonce;
use crate::state::AppState;
use axum::Extension;
use axum::extract::State;
use dto::oauth::response::GoogleOneTapNonceResponse;
use errors::errors::{ErrorResponse, Errors};

/// Issue a single-use Google One Tap nonce.
///
/// The frontend fetches this before initializing Google One Tap and passes it as the
/// `nonce`; the server consumes it during sign-in to prevent ID token replay.
#[utoipa::path(
    get,
    path = "/v0/auth/oauth/google/one-tap/nonce",
    summary = "Issue a Google One Tap nonce",
    description = "Returns a single-use nonce bound to the caller's anonymous id. Pass it to Google One Tap initialization; it is consumed during POST /v0/auth/oauth/google/one-tap/login to block replay.",
    responses(
        (status = 200, description = "A single-use nonce was issued", body = GoogleOneTapNonceResponse),
        (status = 500, description = "Unexpected Redis error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_google_one_tap_nonce(
    State(state): State<AppState>,
    Extension(anonymous): Extension<AnonymousUserContext>,
) -> Result<GoogleOneTapNonceResponse, Errors> {
    let nonce =
        service_issue_google_one_tap_nonce(&state.redis_session, &anonymous.anonymous_user_id)
            .await?;

    Ok(GoogleOneTapNonceResponse { nonce })
}
