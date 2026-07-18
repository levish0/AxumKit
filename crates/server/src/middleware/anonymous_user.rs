use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use config::ServerConfig;
use cookie::SameSite;
use cookie::time::Duration;
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

/// Env-aware cookie name, mirroring `session_cookie_name`/`device_cookie_name`.
///
/// This cookie is the binding value the OAuth state / One-Tap nonce checks rely
/// on to prevent login-CSRF, so its integrity matters like the session cookie's:
/// in production the `__Host-` prefix (or `__Secure-` when a parent
/// `cookie_domain` is configured, since `__Host-` forbids the Domain attribute)
/// makes the browser refuse sibling-origin/insecure injection that could fix the
/// binding to an attacker-chosen value.
pub fn anonymous_cookie_name() -> String {
    let config = ServerConfig::get();
    if config.is_dev {
        "anonymous_user_id".to_string()
    } else if config.cookie_domain.is_some() {
        "__Secure-anonymous_user_id".to_string()
    } else {
        "__Host-anonymous_user_id".to_string()
    }
}

#[derive(Clone)]
pub struct AnonymousUserContext {
    pub anonymous_user_id: String,
}

pub async fn anonymous_user_middleware(
    cookies: Cookies,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let cookie_name = anonymous_cookie_name();

    // Read anonymous_user_id from the cookie
    let (final_anonymous_id, has_anonymous_id) = match cookies.get(&cookie_name) {
        Some(cookie) => (cookie.value().to_string(), true),
        None => (Uuid::now_v7().to_string(), false),
    };

    // Attach the anonymous-user context to request extensions
    req.extensions_mut().insert(AnonymousUserContext {
        anonymous_user_id: final_anonymous_id.clone(),
    });

    let response = next.run(req).await;

    // If the cookie was absent, mint and set a new one
    if !has_anonymous_id {
        let is_dev = ServerConfig::get().is_dev;

        let same_site_attribute = if is_dev {
            SameSite::None
        } else {
            SameSite::Lax
        };

        let config = ServerConfig::get();
        // `Secure` + `Path=/` always set: required by the `__Host-` prefix (which
        // additionally forbids `Domain` — only the `__Secure-` branch adds one).
        let mut cookie_builder = Cookie::build((cookie_name, final_anonymous_id))
            .http_only(true)
            .secure(true)
            .same_site(same_site_attribute)
            .path("/")
            .max_age(Duration::days(365)); // 1 year

        if !is_dev && let Some(ref domain) = config.cookie_domain {
            cookie_builder = cookie_builder.domain(domain);
        }

        cookies.add(cookie_builder.build());
    }

    response
}
