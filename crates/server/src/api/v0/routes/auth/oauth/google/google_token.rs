use crate::service::oauth::google::service_google_token_sign_in;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use dto::auth::response::SessionTokenResponse;
use dto::oauth::request::GoogleTokenRequest;
use dto::oauth::response::OAuthSignInResponse;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

/// Native-app Google sign-in with a provider ID token.
///
/// - Existing user: 200 OK + session token in the body (`SessionTokenResponse`)
/// - New user: 200 OK + pending signup payload (complete via POST /v0/app/auth/complete-signup)
#[utoipa::path(
    post,
    path = "/v0/app/auth/oauth/google/token",
    summary = "Sign in with Google from a native app (provider-token flow)",
    description = "Native-app variant of the Google OAuth flow (allauth `provider/token` pattern). The app obtains a Google ID token via the native Google SDK and submits it here; the server verifies the token directly (JWKS signature, `iss`, `aud` pinned to our client id, `exp`, verified email) with no server-side redirect, state, or browser-cookie binding. An existing linked account receives the opaque session token in the body for `Authorization: Bearer` use. A new identity receives a pending signup token (bound only by its own secrecy) that must be completed via POST /v0/app/auth/complete-signup.",
    request_body = GoogleTokenRequest,
    responses(
        (status = 200, description = "Existing account signed in; session token returned in the body. A NEW identity instead returns 200 with OAuthPendingSignupResponse (profile completion required).", body = SessionTokenResponse),
        (status = 400, description = "Malformed JSON payload, validation error, invalid ID token, or the Google account email is not verified", body = ErrorResponse),
        (status = 409, description = "A local account already uses the same email address", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, JWKS, or Google OAuth error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_google_token_app(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<GoogleTokenRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    let result = service_google_token_sign_in(
        &state.db,
        &state.redis_session,
        &state.http_client,
        &payload.id_token,
        Some(user_agent),
        Some(ip_address),
    )
    .await?;

    // Existing user → session token in body; new user → pending signup payload (already a body).
    OAuthSignInResponse::from_result(result).into_app_response_result()
}
