use crate::config::server_config::ServerConfig;
use crate::errors::errors::Errors;
use axum::http::{HeaderValue, StatusCode, header::SET_COOKIE};
use axum::response::{IntoResponse, Response};
use cookie::{Cookie, SameSite, time::Duration};

pub fn create_login_response(session_id: String) -> Result<Response, Errors> {
    let config = ServerConfig::get();
    let is_dev = config.is_dev;

    let mut response = StatusCode::NO_CONTENT.into_response();

    let same_site_attribute = if is_dev {
        SameSite::None
    } else {
        SameSite::Lax
    };

    let session_cookie = Cookie::build(("session_id", session_id))
        .http_only(true)
        .secure(true)
        .same_site(same_site_attribute)
        .path("/")
        .max_age(Duration::hours(config.auth_session_expire_time))
        .build();

    let header_value = HeaderValue::from_str(&session_cookie.to_string()).map_err(|e| {
        tracing::error!("Failed to create session cookie header: {}", e);
        Errors::SysInternalError("Session cookie header creation failed".to_string())
    })?;

    response.headers_mut().insert(SET_COOKIE, header_value);
    Ok(response)
}
