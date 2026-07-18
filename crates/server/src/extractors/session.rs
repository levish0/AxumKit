use axum::extract::{FromRef, FromRequestParts};
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use std::convert::Infallible;
use tower_cookies::Cookies;

use crate::service::auth::session::SessionService;
use crate::service::auth::session_types::SessionContext;
use crate::state::AppState;
use dto::auth::response::session_cookie_name;
use errors::errors::Errors;

/// Parse `Authorization: Bearer <token>`, returning the raw token.
///
/// The scheme name is case-insensitive per RFC 6750 §2.1.
fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_string())
}

/// Extract the raw session token carried by a request.
///
/// Browser clients send it in the `HttpOnly` session cookie; native-app clients — which have no
/// cookie jar — send the same opaque token as `Authorization: Bearer <token>`. The cookie wins if
/// both are present. The token is never exposed to browser JavaScript (it leaves an `HttpOnly`
/// cookie); apps receive it only in an auth endpoint's response *body* and keep it in OS-backed
/// secure storage — the standard XSS token-theft defense (OWASP Session Management Cheat Sheet).
fn session_token_from_request(cookies: &Cookies, headers: &HeaderMap) -> Option<String> {
    if let Some(cookie) = cookies.get(&session_cookie_name()) {
        return Some(cookie.value().to_string());
    }
    bearer_token(headers)
}

/// Resolve the session from a request, delegating validation to [`SessionService::resolve_session`].
///
/// Authentication is whichever credential the request carries — the session cookie (browser) or a
/// `Authorization: Bearer` token (native app); both are the same opaque session token, so
/// validation is identical. Every request validates against Redis + the DB here.
/// `Ok(None)` means no usable session; errors mirror the service.
pub async fn resolve_session_from_request(
    cookies: &Cookies,
    headers: &HeaderMap,
    app_state: &AppState,
) -> Result<Option<SessionContext>, Errors> {
    let Some(session_token) = session_token_from_request(cookies, headers) else {
        return Ok(None);
    };
    SessionService::resolve_session(&app_state.redis_session, &app_state.db, &session_token).await
}

/// Required session extractor - fails with error if session is not present or invalid
///
/// Use this in handlers that require authentication:
/// ```ignore
/// pub async fn protected_handler(
///     RequiredSession(session): RequiredSession,
/// ) { ... }
/// ```
#[derive(Debug, Clone)]
pub struct RequiredSession(pub SessionContext);

impl<S> FromRequestParts<S> for RequiredSession
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = Errors;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookies = parts.extensions.get::<Cookies>().ok_or_else(|| {
            tracing::warn!("RequiredSession: Cookies extension not found in request");
            Errors::UserUnauthorized
        })?;
        let app_state = AppState::from_ref(state);
        resolve_session_from_request(cookies, &parts.headers, &app_state)
            .await?
            .map(RequiredSession)
            .ok_or(Errors::UserUnauthorized)
    }
}

/// Optional session extractor - returns None if session is not present or invalid
///
/// Use this in handlers that work with or without authentication:
/// ```ignore
/// pub async fn public_handler(
///     OptionalSession(session): OptionalSession,
/// ) {
///     if let Some(session) = session {
///         // Authenticated user
///     } else {
///         // Anonymous user
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OptionalSession(pub Option<SessionContext>);

impl<S> FromRequestParts<S> for OptionalSession
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = Infallible; // Never fails!

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Validate the credential ourselves, treating any error as anonymous.
        let Some(cookies) = parts.extensions.get::<Cookies>() else {
            return Ok(OptionalSession(None));
        };
        let app_state = AppState::from_ref(state);
        let context = resolve_session_from_request(cookies, &parts.headers, &app_state)
            .await
            .ok()
            .flatten();
        Ok(OptionalSession(context))
    }
}

#[cfg(test)]
mod tests {
    use super::bearer_token;
    use axum::http::HeaderMap;
    use axum::http::header::AUTHORIZATION;

    fn headers_with_auth(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, value.parse().unwrap());
        headers
    }

    #[test]
    fn extracts_bearer_token() {
        let headers = headers_with_auth("Bearer abc123");
        assert_eq!(bearer_token(&headers).as_deref(), Some("abc123"));
    }

    #[test]
    fn scheme_is_case_insensitive() {
        // RFC 6750 §2.1: the "Bearer" scheme name is case-insensitive.
        let headers = headers_with_auth("bearer abc123");
        assert_eq!(bearer_token(&headers).as_deref(), Some("abc123"));
    }

    #[test]
    fn rejects_non_bearer_scheme() {
        let headers = headers_with_auth("Basic dXNlcjpwYXNz");
        assert_eq!(bearer_token(&headers), None);
    }

    #[test]
    fn rejects_empty_token() {
        let headers = headers_with_auth("Bearer    ");
        assert_eq!(bearer_token(&headers), None);
    }

    #[test]
    fn none_when_header_absent() {
        assert_eq!(bearer_token(&HeaderMap::new()), None);
    }
}
