use crate::dto::auth::internal::refresh_token::RefreshTokenContext;
use crate::dto::auth::request::login::AuthLoginRequest;
use crate::dto::auth::response::jwt::AuthJWTResponse;
use crate::middleware::auth::refresh_jwt_auth;
use crate::service::auth::auth::{service_login, service_refresh};
use crate::service::error::errors::Errors;
use crate::service::validator::json_validator::ValidatedJson;
use crate::state::AppState;
use axum::routing::post;
use axum::{Extension, Router, extract::State};
use axum_extra::TypedHeader;
use headers::UserAgent;

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route(
            "/auth/refresh",
            post(refresh).route_layer(axum::middleware::from_fn(refresh_jwt_auth)),
        )
}

#[utoipa::path(
    post,
    path = "/v0/auth/login",
    request_body = AuthLoginRequest,
    responses(
        (status = StatusCode::OK, description = "Login successful", body = AuthJWTResponse),
        (status = StatusCode::UNAUTHORIZED, description = "Invalid credentials"),
        (status = StatusCode::BAD_REQUEST, description = "Invalid request"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn login(
    user_agent: Option<TypedHeader<UserAgent>>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<AuthLoginRequest>,
) -> Result<AuthJWTResponse, Errors> {
    let res = service_login(&state.conn, payload).await?;

    Ok(AuthJWTResponse {
        access_token: res.access_token,
        cookie_refresh_token: res.cookie_refresh_token,
    })
}

#[utoipa::path(
    post,
    path = "/v0/auth/refresh",
    responses(
        (status = StatusCode::OK, description = "Token refresh successful", body = AuthJWTResponse),
        (status = StatusCode::UNAUTHORIZED, description = "Invalid or expired refresh token"),
        (status = StatusCode::BAD_REQUEST, description = "Invalid request"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn refresh(
    user_agent: Option<TypedHeader<UserAgent>>,
    State(state): State<AppState>,
    Extension(ctx): Extension<RefreshTokenContext>,
) -> Result<AuthJWTResponse, Errors> {
    let refresh_token = ctx.token;
    let refresh_token_claims = ctx.claims;
    let res = service_refresh(&state.conn, refresh_token, refresh_token_claims).await?;

    Ok(AuthJWTResponse {
        access_token: res.access_token,
        cookie_refresh_token: res.cookie_refresh_token,
    })
}