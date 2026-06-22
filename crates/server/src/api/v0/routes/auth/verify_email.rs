use crate::service::auth::session::SessionService;
use crate::service::auth::verify_email::service_verify_email;
use crate::service::user::utils::spawn_index_user;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum_extra::{TypedHeader, headers::UserAgent};
use dto::auth::request::VerifyEmailRequest;
use dto::auth::response::create_login_response;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

#[utoipa::path(
    post,
    path = "/v0/auth/verify-email",
    summary = "Complete an email signup with a verification token",
    description = "Consumes the pending email verification token, creates the user account if the email and handle are still available, schedules background indexing, and issues a session cookie. The token is only cleaned up after the database commit succeeds.",
    request_body = VerifyEmailRequest,
    responses(
        (status = 204, description = "Verification token accepted, account created, and session cookie issued"),
        (status = 400, description = "Malformed JSON payload, validation error, or invalid verification token", body = ErrorResponse),
        (status = 409, description = "The email or handle became unavailable before the account was created", body = ErrorResponse),
        (status = 500, description = "Unexpected database or Redis error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_verify_email(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<VerifyEmailRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    let user_id = service_verify_email(&state.db, &state.redis_session, &payload.token).await?;

    spawn_index_user(&state.worker, user_id);

    let (raw_token, _session) = SessionService::create_session(
        &state.redis_session,
        user_id.to_string(),
        Some(user_agent),
        Some(ip_address),
    )
    .await?;

    create_login_response(raw_token, false)
}
