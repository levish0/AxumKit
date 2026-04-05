use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::google::service_google_one_tap_sign_in;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::Extension;
use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use axumkit_dto::oauth::request::google::GoogleOneTapLoginRequest;
use axumkit_dto::oauth::response::{OAuthPendingSignupResponse, OAuthSignInResponse};
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

#[utoipa::path(
    post,
    path = "/v0/auth/oauth/google/one-tap/login",
    summary = "Sign in with Google One Tap",
    description = "Validates the Google ID token on the server. Existing linked accounts receive a session immediately. New identities receive a pending signup token that must be completed via POST /v0/auth/complete-signup.",
    request_body = GoogleOneTapLoginRequest,
    responses(
        (status = 200, description = "Google identity was accepted but profile completion is still required", body = OAuthPendingSignupResponse),
        (status = 204, description = "Google identity matched an existing account and a session cookie was issued"),
        (status = 400, description = "Malformed JSON payload, validation error, invalid ID token, or the Google account email is not verified", body = ErrorResponse),
        (status = 409, description = "A local account already uses the same email address", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, JWKS, or Google OAuth error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_google_one_tap_login(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Extension(anonymous): Extension<AnonymousUserContext>,
    ValidatedJson(payload): ValidatedJson<GoogleOneTapLoginRequest>,
) -> Result<Response, Errors> {
    let user_agent_str = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    let result = service_google_one_tap_sign_in(
        &state.db,
        &state.redis_session,
        &state.http_client,
        &payload.credential,
        &anonymous.anonymous_user_id,
        Some(user_agent_str),
        Some(ip_address),
    )
    .await?;

    OAuthSignInResponse::from_result(result).into_response_result()
}
