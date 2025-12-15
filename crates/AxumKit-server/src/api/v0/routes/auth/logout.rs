use crate::dto::auth::create_logout_response;
use crate::dto::auth::internal::session::SessionContext;
use crate::errors::errors::Errors;
use crate::extractors::session::RequiredSession;
use crate::service::auth::logout::service_logout;
use crate::state::AppState;
use axum::{
    extract::{Extension, State},
    response::Response,
};

#[utoipa::path(
    post,
    path = "/v0/auth/logout",
    responses(
        (status = 204, description = "Logout successful"),
        (status = 401, description = "Unauthorized")
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
    service_logout(&state.redis_client, &session_context.session_id).await?;

    // 쿠키 클리어하는 204 응답 반환
    create_logout_response()
}
