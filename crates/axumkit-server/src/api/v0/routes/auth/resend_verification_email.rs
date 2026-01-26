use crate::extractors::RequiredSession;
use crate::service::auth::resend_verification_email::service_resend_verification_email;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/resend-verification-email",
    responses(
        (status = 204, description = "Verification email sent successfully"),
        (status = 401, description = "Unauthorized - Invalid or expired session, or OAuth user without password"),
        (status = 404, description = "Not Found - User not found"),
        (status = 409, description = "Conflict - Email already verified"),
        (status = 500, description = "Internal Server Error - Database or Redis error"),
        (status = 502, description = "Bad Gateway - Worker service request failed or returned invalid response"),
        (status = 503, description = "Service Unavailable - Worker service connection failed")
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth"
)]
pub async fn auth_resend_verification_email(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<impl IntoResponse, Errors> {
    service_resend_verification_email(
        &state.write_db,
        &state.redis_session,
        &state.worker,
        session_context.user_id,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
