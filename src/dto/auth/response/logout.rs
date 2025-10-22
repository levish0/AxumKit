use crate::errors::errors::Errors;
use axum::http::{HeaderValue, StatusCode, header::SET_COOKIE};
use axum::response::{IntoResponse, Response};
use cookie::{Cookie, time::Duration};

pub fn create_logout_response() -> Result<Response, Errors> {
    let mut response = StatusCode::NO_CONTENT.into_response();

    // 세션 쿠키 삭제 (만료 시간을 과거로 설정)
    let clear_cookie = Cookie::build(("session_id", ""))
        .http_only(true)
        .secure(true)
        .path("/")
        .max_age(Duration::seconds(0))
        .build();

    let header_value = HeaderValue::from_str(&clear_cookie.to_string()).map_err(|e| {
        tracing::error!("Failed to create logout cookie header: {}", e);
        Errors::SysInternalError("Logout cookie header creation failed".to_string())
    })?;

    response.headers_mut().insert(SET_COOKIE, header_value);
    Ok(response)
}
