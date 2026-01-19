use crate::service::auth::confirm_email_change::service_confirm_email_change;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axumkit_dto::auth::request::ConfirmEmailChangeRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/confirm-email-change",
    request_body = ConfirmEmailChangeRequest,
    responses(
        (status = 204, description = "Email changed successfully"),
        (status = 400, description = "Bad request - Invalid or expired token"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Auth"
)]
pub async fn auth_confirm_email_change(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ConfirmEmailChangeRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_confirm_email_change(&state.conn, &state.redis_session, &payload.token).await?;

    Ok(StatusCode::NO_CONTENT)
}
