use crate::config::HttpSettings;
use axum::{
    http::{HeaderValue, header},
    response::Response,
};
use cookie::{Cookie, SameSite, time::Duration};

pub fn set_cookie(
    response: &mut Response,
    settings: &HttpSettings,
    name: &'static str,
    value: String,
    lifetime_seconds: i64,
) {
    let cookie = Cookie::build((name, value))
        .path("/")
        .http_only(true)
        .secure(settings.secure_cookies)
        .same_site(SameSite::Lax)
        .max_age(Duration::seconds(lifetime_seconds))
        .build();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).expect("generated cookie"),
    );
}

pub fn clear_session(response: &mut Response, settings: &HttpSettings) {
    set_cookie(
        response,
        settings,
        settings.session_cookie_name(),
        String::new(),
        0,
    );
}
