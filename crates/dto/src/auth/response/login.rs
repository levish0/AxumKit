use axum::http::{HeaderValue, StatusCode, header::SET_COOKIE};
use axum::response::{IntoResponse, Response};
use config::ServerConfig;
use cookie::{Cookie, SameSite, time::Duration};
use errors::errors::Errors;

pub fn create_login_response(session_id: String, remember_me: bool) -> Result<Response, Errors> {
    let config = ServerConfig::get();
    let is_dev = config.is_dev;

    let mut response = StatusCode::NO_CONTENT.into_response();

    let same_site_attribute = if is_dev {
        SameSite::None
    } else {
        SameSite::Lax
    };

    let mut cookie_builder = Cookie::build(("session_id", session_id))
        .http_only(true)
        .secure(true)
        .same_site(same_site_attribute)
        .path("/");

    // Set cookie domain for cross-subdomain sharing (production only)
    if !is_dev {
        if let Some(ref domain) = config.cookie_domain {
            cookie_builder = cookie_builder.domain(domain);
        }
    }

    // remember_me=true: persistent cookie (maximum session lifetime)
    // remember_me=false: session cookie (deleted when browser is closed)
    if remember_me {
        cookie_builder =
            cookie_builder.max_age(Duration::hours(config.auth_session_max_lifetime_hours));
    }

    let session_cookie = cookie_builder.build();

    let header_value = HeaderValue::from_str(&session_cookie.to_string()).map_err(|e| {
        tracing::error!("Failed to create session cookie header: {}", e);
        Errors::SysInternalError("Session cookie header creation failed".to_string())
    })?;

    response.headers_mut().append(SET_COOKIE, header_value);
    Ok(response)
}
