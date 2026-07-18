use crate::extractors::RequiredSession;
use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::github::service_link_github_oauth;
use crate::state::AppState;
use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use dto::oauth::request::link::GithubLinkRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

/// Links GitHub OAuth to the current account.
#[utoipa::path(
    post,
    path = "/v0/auth/oauth/github/link",
    summary = "Link a GitHub account to the current user",
    description = "Exchanges the GitHub authorization code, validates the single-use state created by the GitHub authorize endpoint, and stores the GitHub identity on the authenticated account. The state is bound to the same anonymous browser context that started the link flow.",
    request_body = GithubLinkRequest,
    responses(
        (status = 204, description = "GitHub account was linked to the current user"),
        (status = 400, description = "Malformed JSON payload, validation error, or invalid or expired state or code", body = ErrorResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 409, description = "The GitHub identity is already linked to this account or another account", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, or GitHub OAuth error", body = ErrorResponse)
    ),
    tag = "Auth",
    security(
        ("session_id_cookie" = [])
    )
)]
pub async fn auth_github_link(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    Extension(anonymous): Extension<AnonymousUserContext>,
    ValidatedJson(payload): ValidatedJson<GithubLinkRequest>,
) -> Result<StatusCode, Errors> {
    service_link_github_oauth(
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
