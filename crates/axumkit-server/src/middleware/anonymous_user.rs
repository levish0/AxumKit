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
    // 쿠키에서 anonymous_user_id 확인
    let (final_anonymous_id, has_anonymous_id) = match cookies.get(ANONYMOUS_USER_COOKIE_NAME) {
        Some(cookie) => (cookie.value().to_string(), true),
        None => (Uuid::now_v7().to_string(), false),
    };

    // Extension에 익명 사용자 컨텍스트 추가
    req.extensions_mut().insert(AnonymousUserContext {
        anonymous_user_id: final_anonymous_id.clone(),
    });

    let response = next.run(req).await;

    // 쿠키가 없었다면 새로 생성해서 설정
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
            .max_age(Duration::days(365)); // 1년

        if !is_dev {
            if let Some(ref domain) = config.cookie_domain {
                cookie_builder = cookie_builder.domain(domain);
            }
        }

        cookies.add(cookie_builder.build());
    }

    response
}
