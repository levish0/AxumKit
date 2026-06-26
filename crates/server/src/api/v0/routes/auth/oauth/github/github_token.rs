use crate::service::oauth::github::service_github_token_sign_in;
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
use dto::oauth::request::GithubTokenRequest;
use dto::oauth::response::OAuthSignInResponse;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

/// Native-app GitHub sign-in with a provider access token.
///
/// - Existing user: 200 OK + session token in the body (`SessionTokenResponse`)
/// - New user: 200 OK + pending signup payload (complete via POST /v0/app/auth/complete-signup)
#[utoipa::path(
    post,
    path = "/v0/app/auth/oauth/github/token",
    summary = "Sign in with GitHub from a native app (provider-token flow)",
    description = "Native-app variant of the GitHub OAuth flow (allauth `provider/token` pattern). The app submits the GitHub access token it obtained through its own in-app authorization. Because a bare access token has no verifiable audience, the server validates it against GitHub's token introspection endpoint (`POST /applications/{client_id}/token`, authenticated with our client id + secret), which confirms the token was issued for our OAuth app — a token minted for another app is rejected. The verified primary email is then taken from `/user/emails`. An existing linked account receives the opaque session token in the body for `Authorization: Bearer` use. A new identity receives a pending signup token (bound only by its own secrecy) completed via POST /v0/app/auth/complete-signup.",
    request_body = GithubTokenRequest,
    responses(
        (status = 200, description = "Existing account signed in; session token returned in the body. A NEW identity instead returns 200 with OAuthPendingSignupResponse (profile completion required).", body = SessionTokenResponse),
        (status = 400, description = "Malformed JSON payload, validation error, an access token not issued for our app, or no verified primary email", body = ErrorResponse),
        (status = 409, description = "A local account already uses the same email address", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, or GitHub API error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_github_token_app(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<GithubTokenRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    let result = service_github_token_sign_in(
        &state.db,
        &state.redis_session,
        &state.http_client,
        &payload.access_token,
        Some(user_agent),
        Some(ip_address),
    )
    .await?;

    // Existing user → session token in body; new user → pending signup payload (already a body).
    OAuthSignInResponse::from_result(result).into_app_response_result()
}
