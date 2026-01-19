use crate::service::auth::forgot_password::service_forgot_password;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axumkit_dto::auth::request::ForgotPasswordRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 204, description = "Password reset email sent if account exists"),
        (status = 400, description = "Bad request - Invalid JSON or validation error"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Auth"
)]
pub async fn auth_forgot_password(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ForgotPasswordRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_forgot_password(
        &state.conn,
        &state.redis_session,
        &state.worker,
        &payload.email,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
