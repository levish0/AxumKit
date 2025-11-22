use crate::dto::auth::internal::session::SessionContext;
use crate::errors::errors::Errors;
use crate::service::auth::session::SessionService;
use crate::state::AppState;
use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use tower_cookies::Cookies;
use uuid::Uuid;

const SESSION_COOKIE_NAME: &str = "session_id";

// DEPRECATED: Use RequiredSession extractor instead
#[allow(dead_code)]
pub async fn session_auth(
    State(state): State<AppState>,
    cookies: Cookies,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, Errors> {
    // 쿠키에서 session_id 추출
    let session_id = cookies
        .get(SESSION_COOKIE_NAME)
        .map(|cookie| cookie.value().to_string())
        .ok_or(Errors::UserUnauthorized)?;

    // Redis에서 세션 조회해서 user_id 추출
    let session = SessionService::get_session(&state.redis_client, &session_id)
        .await?
        .ok_or(Errors::UserUnauthorized)?;

    // user_id를 UUID로 파싱
    let user_id = Uuid::parse_str(&session.user_id).map_err(|_| Errors::SessionInvalidUserId)?;

    // Redis Session 데이터에서 user_id와 session_id만 사용해서 SessionContext 구성
    req.extensions_mut().insert(SessionContext {
        user_id,
        session_id,
    });

    Ok(next.run(req).await)
}

/// DEPRECATED: Use OptionalSession extractor instead
#[allow(dead_code)]
pub async fn optional_session_auth(
    State(state): State<AppState>,
    cookies: Cookies,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // 쿠키에서 session_id 추출 시도
    if let Some(cookie) = cookies.get(SESSION_COOKIE_NAME) {
        let session_id = cookie.value().to_string();

        // Redis에서 세션 조회 시도
        if let Ok(Some(session)) =
            SessionService::get_session(&state.redis_client, &session_id).await
        {
            // user_id 파싱 시도
            if let Ok(user_id) = Uuid::parse_str(&session.user_id) {
                // 성공하면 SessionContext 추가
                req.extensions_mut().insert(SessionContext {
                    user_id,
                    session_id,
                });
            }
        }
    }

    // 실패해도 에러 반환하지 않고 계속 진행
    next.run(req).await
}
