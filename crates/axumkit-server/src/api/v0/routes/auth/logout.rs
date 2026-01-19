use crate::extractors::RequiredSession;
use crate::service::auth::logout::service_logout;
use crate::state::AppState;
use axum::{extract::State, response::Response};
use axumkit_dto::auth::response::create_logout_response;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/logout",
    responses(
        (status = 204, description = "Logout successful"),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 500, description = "Internal Server Error - Redis error")
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth"
)]
pub async fn auth_logout(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<Response, Errors> {
    // 로그아웃 처리
    service_logout(&state.redis_session, &session_context.session_id).await?;

    // 쿠키 클리어하는 204 응답 반환
    create_logout_response()
}
