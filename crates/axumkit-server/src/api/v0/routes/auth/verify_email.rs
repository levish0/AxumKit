use crate::service::auth::verify_email::service_verify_email;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axumkit_dto::auth::request::VerifyEmailRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/verify-email",
    request_body = VerifyEmailRequest,
    responses(
        (status = 204, description = "Email verified successfully"),
        (status = 400, description = "Bad request - Invalid JSON, validation error, invalid or expired token"),
        (status = 404, description = "Not Found - User not found"),
        (status = 409, description = "Conflict - Email already verified"),
        (status = 500, description = "Internal Server Error - Database or Redis error")
    ),
    tag = "Auth"
)]
pub async fn auth_verify_email(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<VerifyEmailRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_verify_email(&state.write_db, &state.redis_session, &payload.token).await?;

    Ok(StatusCode::NO_CONTENT)
}
