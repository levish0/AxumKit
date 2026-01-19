use crate::service::auth::totp::service_totp_verify;
use crate::state::AppState;
use axum::extract::State;
use axum::response::Response;
use axumkit_dto::auth::request::TotpVerifyRequest;
use axumkit_dto::auth::response::create_login_response;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/totp/verify",
    request_body = TotpVerifyRequest,
    responses(
        (status = 204, description = "TOTP verified, login successful"),
        (status = 400, description = "Invalid TOTP code or temp token"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_verify(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<TotpVerifyRequest>,
) -> Result<Response, Errors> {
    let result = service_totp_verify(
        &state.conn,
        &state.redis_session,
        &payload.temp_token,
        &payload.code,
    )
    .await?;

    create_login_response(result.session_id, result.remember_me)
}
