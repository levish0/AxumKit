use crate::payload::auth_payload::{AuthLoginAccessTokenResponse, AuthLoginRequest};
use crate::service::auth::auth::service_login;
use crate::service::error::errors::Errors;
use crate::service::validator::json_validator::ValidatedJson;
use crate::state::AppState;
use axum::routing::post;
use axum::{Router, extract::State};

pub fn auth_routes() -> Router<AppState> {
    Router::new().route("/auth/login", post(login))
}

#[utoipa::path(
    post,
    path = "/v0/auth/login",
    request_body = AuthLoginRequest,
    responses(
        (status = StatusCode::OK, description = "Login successful", body = AuthLoginAccessTokenResponse),
        (status = StatusCode::UNAUTHORIZED, description = "Invalid credentials"),
        (status = StatusCode::BAD_REQUEST, description = "Invalid request"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal server error")
    ),
    tag = "Auth"
)]
#[axum_macros::debug_handler]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<AuthLoginRequest>,
) -> Result<AuthLoginAccessTokenResponse, Errors> {
    let res = service_login(&state.conn, payload).await?;

    Ok(AuthLoginAccessTokenResponse {
        access_token: res.access_token,
        refresh_token: res.refresh_token,
    })
}
