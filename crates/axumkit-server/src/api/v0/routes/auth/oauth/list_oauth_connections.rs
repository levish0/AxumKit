use crate::extractors::RequiredSession;
use crate::service::oauth::list_connections::service_list_oauth_connections;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::oauth::response::OAuthConnectionListResponse;
use axumkit_errors::errors::Errors;

/// 현재 사용자의 OAuth 연결 목록을 조회합니다.
#[utoipa::path(
    get,
    path = "/v0/auth/oauth/connections",
    responses(
        (status = 200, description = "OAuth connections list", body = OAuthConnectionListResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 500, description = "Internal Server Error - Database error")
    ),
    tag = "Auth",
    security(
        ("session_id_cookie" = [])
    )
)]
pub async fn list_oauth_connections(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<OAuthConnectionListResponse, Errors> {
    let result = service_list_oauth_connections(&state.conn, session_context.user_id).await?;

    Ok(result)
}
