use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use axumkit_config::ServerConfig;
use cookie::SameSite;
use cookie::time::Duration;
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

pub const ANONYMOUS_USER_COOKIE_NAME: &str = "anonymous_user_id";

#[derive(Clone)]
pub struct AnonymousUserContext {
    pub anonymous_user_id: String,
}

pub async fn anonymous_user_middleware(
    cookies: Cookies,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // Check for anonymous_user_id in cookies
    let (final_anonymous_id, has_anonymous_id) = match cookies.get(ANONYMOUS_USER_COOKIE_NAME) {
        Some(cookie) => (cookie.value().to_string(), true),
        None => (Uuid::now_v7().to_string(), false),
    };

    // Add anonymous user context to extensions
    req.extensions_mut().insert(AnonymousUserContext {
        anonymous_user_id: final_anonymous_id.clone(),
    });

    let response = next.run(req).await;

    // If cookie was missing, create and set a new one
    if !has_anonymous_id {
        let is_dev = ServerConfig::get().is_dev;

        let same_site_attribute = if is_dev {
            SameSite::None
        } else {
            SameSite::Lax
        };

        let config = ServerConfig::get();
        let mut cookie_builder = Cookie::build((ANONYMOUS_USER_COOKIE_NAME, final_anonymous_id))
            .http_only(true)
            .secure(true)
            .same_site(same_site_attribute)
            .path("/")
            .max_age(Duration::days(365)); // 1 year

        if !is_dev {
            if let Some(ref domain) = config.cookie_domain {
                cookie_builder = cookie_builder.domain(domain);
            }
        }

        cookies.add(cookie_builder.build());
    }

    response
}
