use crate::service::auth::LoginResult;
use crate::service::auth::login::service_login;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{
    extract::{ConnectInfo, State},
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use axumkit_dto::auth::request::LoginRequest;
use axumkit_dto::auth::response::TotpRequiredResponse;
use axumkit_dto::auth::response::create_login_response;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;
use std::net::SocketAddr;

#[utoipa::path(
    post,
    path = "/v0/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 204, description = "Login successful"),
        (status = 202, description = "TOTP required", body = TotpRequiredResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error"),
        (status = 401, description = "Unauthorized - Invalid credentials or password not set"),
        (status = 404, description = "Not Found - User not found"),
        (status = 500, description = "Internal Server Error - Database or Redis error")
    ),
    tag = "Auth"
)]
pub async fn auth_login(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    // 로그인 처리
    let result = service_login(
        &state.write_db,
        &state.redis_session,
        payload,
        Some(user_agent),
        Some(ip_address),
    )
    .await?;

    match result {
        LoginResult::SessionCreated {
            session_id,
            remember_me,
        } => {
            // 쿠키 설정하는 204 응답 반환
            create_login_response(session_id, remember_me)
        }
        LoginResult::TotpRequired(temp_token) => {
            // TOTP 필요: 202 + temp_token 반환
            Ok(TotpRequiredResponse { temp_token }.into_response())
        }
    }
}
