use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::complete_signup::service_complete_signup;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::Extension;
use axum::http::HeaderMap;
use axum::{
    extract::{ConnectInfo, State},
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use axumkit_dto::auth::request::CompleteSignupRequest;
use axumkit_dto::auth::response::create_login_response;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;
use std::net::SocketAddr;

///
#[utoipa::path(
    post,
    path = "/v0/auth/complete-signup",
    request_body = CompleteSignupRequest,
    responses(
        (status = 204, description = "Signup completed successfully"),
        (status = 400, description = "Bad request - Invalid JSON or validation error"),
        (status = 401, description = "Unauthorized - Token expired or invalid"),
        (status = 409, description = "Conflict - Handle or email already exists"),
        (status = 500, description = "Internal Server Error - Database or Redis error")
    ),
    tag = "Auth"
)]
pub async fn auth_complete_signup(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Extension(anonymous): Extension<AnonymousUserContext>,
    ValidatedJson(payload): ValidatedJson<CompleteSignupRequest>,
) -> Result<Response, Errors> {
    let user_agent_str = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    let session_id = service_complete_signup(
        &state.write_db,
        &state.redis_session,
        &state.http_client,
        &state.r2_client,
        &state.worker,
        &payload.pending_token,
        &payload.handle,
        &payload.display_name,
        &anonymous.anonymous_user_id,
        Some(user_agent_str),
        Some(ip_address),
    )
    .await?;

    // Return 204 with login cookie (session max lifetime is server-configured).
    create_login_response(session_id, true)
}
