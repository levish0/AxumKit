use crate::extractors::RequiredSession;
use crate::service::oauth::link_github::service_link_github_oauth;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axumkit_dto::oauth::request::link::GithubLinkRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

/// GitHub OAuth를 현재 계정에 연결합니다.
#[utoipa::path(
    post,
    path = "/v0/auth/oauth/github/link",
    request_body = GithubLinkRequest,
    responses(
        (status = 204, description = "OAuth linked successfully"),
        (status = 400, description = "Bad request - Invalid JSON, validation error, invalid or expired state/code"),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 409, description = "Conflict - OAuth account already linked to this or another user"),
        (status = 500, description = "Internal Server Error - Database, Redis, or OAuth provider error")
    ),
    tag = "Auth",
    security(
        ("session_id_cookie" = [])
    )
)]
pub async fn auth_github_link(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedJson(payload): ValidatedJson<GithubLinkRequest>,
) -> Result<StatusCode, Errors> {
    service_link_github_oauth(
        &state.write_db,
        &state.redis_session,
        &state.http_client,
        session_context.user_id,
        &payload.code,
        &payload.state,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
