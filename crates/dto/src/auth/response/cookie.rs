//! Session cookie naming.

use config::ServerConfig;

/// The session cookie name, with the strongest applicable name prefix.
///
/// Cookie name prefixes (`__Host-`, `__Secure-`) make the browser refuse the
/// cookie unless its security attributes match, blocking cookie-injection /
/// fixation from a sibling origin. They require the cookie to be `Secure`, so they
/// only apply to the HTTPS production cookie:
/// - prod, no `Domain` (host-only)         → `__Host-session_id` (also needs `Path=/`)
/// - prod, with `Domain` (cross-subdomain) → `__Secure-session_id`
/// - dev (plain-HTTP localhost)            → `session_id` (prefixes would be rejected)
///
/// The setters (`create_login_response` / `create_logout_response`) and the
/// session extractor all derive the name here, so they always agree.
pub fn session_cookie_name() -> String {
    let config = ServerConfig::get();
    if config.is_dev {
        "session_id".to_string()
    } else if config.cookie_domain.is_some() {
        "__Secure-session_id".to_string()
    } else {
        "__Host-session_id".to_string()
    }
}
