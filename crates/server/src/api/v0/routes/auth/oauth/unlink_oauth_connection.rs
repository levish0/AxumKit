use crate::extractors::RequiredSession;
use crate::service::oauth::unlink_connection::service_unlink_oauth;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use dto::oauth::request::unlink::UnlinkOAuthRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

/// Unlinks an OAuth connection.
#[utoipa::path(
    post,
    path = "/v0/auth/oauth/connections/unlink",
    summary = "Unlink an OAuth provider from the current account",
    description = "Removes one linked OAuth provider from the authenticated account. This endpoint refuses to unlink the last remaining sign-in method when the account has no password set.",
    request_body = UnlinkOAuthRequest,
    responses(
        (status = 204, description = "The requested OAuth provider was unlinked"),
        (status = 400, description = "Malformed JSON payload, validation error, or unlinking this provider would remove the last sign-in method", body = ErrorResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 404, description = "The requested OAuth provider is not linked to this account", body = ErrorResponse),
        (status = 500, description = "Unexpected database error", body = ErrorResponse)
    ),
    tag = "Auth",
    security(
        ("session_id_cookie" = [])
    )
)]
pub async fn unlink_oauth_connection(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedJson(payload): ValidatedJson<UnlinkOAuthRequest>,
) -> Result<StatusCode, Errors> {
    service_unlink_oauth(&state.db, session_context.user_id, payload.provider).await?;

    Ok(StatusCode::NO_CONTENT)
}
