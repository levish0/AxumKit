use crate::extractors::RequiredSession;
use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::google::service_link_google_oauth;
use crate::state::AppState;
use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use dto::oauth::request::link::GoogleLinkRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

/// Links Google OAuth to the current account.
#[utoipa::path(
    post,
    path = "/v0/auth/oauth/google/link",
    summary = "Link a Google account to the current user",
    description = "Exchanges the Google authorization code, validates the single-use state created by the Google authorize endpoint, and stores the Google identity on the authenticated account. The state is bound to the same anonymous browser context that started the link flow.",
    request_body = GoogleLinkRequest,
    responses(
        (status = 204, description = "Google account was linked to the current user"),
        (status = 400, description = "Malformed JSON payload, validation error, invalid or expired state or code, or the Google account email is not verified", body = ErrorResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 409, description = "The Google identity is already linked to this account or another account", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, or Google OAuth error", body = ErrorResponse)
    ),
    tag = "Auth",
    security(
        ("session_id_cookie" = [])
    )
)]
pub async fn auth_google_link(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    Extension(anonymous): Extension<AnonymousUserContext>,
    ValidatedJson(payload): ValidatedJson<GoogleLinkRequest>,
) -> Result<StatusCode, Errors> {
    service_link_google_oauth(
        &state.db,
        &state.redis_session,
        &state.http_client,
        session_context.user_id,
        &payload.code,
        &payload.state,
        &anonymous.anonymous_user_id,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
