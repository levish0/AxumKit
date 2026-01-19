use axum::http::{HeaderValue, StatusCode, header::SET_COOKIE};
use axum::response::{IntoResponse, Response};
use axumkit_config::ServerConfig;
use axumkit_errors::errors::Errors;
use cookie::{Cookie, SameSite, time::Duration};

pub fn create_logout_response() -> Result<Response, Errors> {
    let config = ServerConfig::get();
    let is_dev = config.is_dev;
    let mut response = StatusCode::NO_CONTENT.into_response();

    let same_site_attribute = if is_dev {
        SameSite::None
    } else {
        SameSite::Lax
    };

    // 세션 쿠키 삭제 (만료 시간을 과거로 설정)
    let mut cookie_builder = Cookie::build(("session_id", ""))
        .http_only(true)
        .secure(true)
        .same_site(same_site_attribute)
        .path("/")
        .max_age(Duration::seconds(0));

    if !is_dev {
        if let Some(ref domain) = config.cookie_domain {
            cookie_builder = cookie_builder.domain(domain);
        }
    }

    let clear_cookie = cookie_builder.build();

    let header_value = HeaderValue::from_str(&clear_cookie.to_string()).map_err(|e| {
        tracing::error!("Failed to create logout cookie header: {}", e);
        Errors::SysInternalError("Logout cookie header creation failed".to_string())
    })?;

    response.headers_mut().insert(SET_COOKIE, header_value);
    Ok(response)
}
