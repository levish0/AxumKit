use crate::dto::auth::{LoginRequest, create_login_response};
use crate::errors::errors::Errors;
use crate::service::auth::login::service_login;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use crate::utils::validator::json_validator::ValidatedJson;
use axum::http::HeaderMap;
use axum::{
    extract::{ConnectInfo, State},
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use std::net::SocketAddr;

#[utoipa::path(
    post,
    path = "/v0/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 204, description = "Login successful"),
        (status = 401, description = "Invalid credentials"),
        (status = 400, description = "Validation error")
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
    let session_id = service_login(
        &state.conn,
        &state.redis_client,
        payload,
        Some(user_agent),
        Some(ip_address),
    )
    .await?;

    // 쿠키 설정하는 204 응답 반환
    create_login_response(session_id)
}
