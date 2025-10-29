use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use std::convert::Infallible;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::dto::auth::internal::session::SessionContext;
use crate::errors::errors::Errors;
use crate::service::auth::session::SessionService;
use crate::state::AppState;

pub const SESSION_COOKIE_NAME: &str = "session_id";

/// Required session extractor - fails with error if session is not present or invalid
///
/// Use this in handlers that require authentication:
/// ```
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
        // Extract AppState
        let app_state = AppState::from_ref(state);

        // Extract Cookies
        let cookies = parts
            .extensions
            .get::<Cookies>()
            .ok_or(Errors::UserUnauthorized)?;

        // Extract session_id from cookie
        let session_id = cookies
            .get(SESSION_COOKIE_NAME)
            .map(|cookie| cookie.value().to_string())
            .ok_or(Errors::UserUnauthorized)?;

        // Get session from Redis
        let session = SessionService::get_session(&app_state.redis_client, &session_id)
            .await?
            .ok_or(Errors::UserUnauthorized)?;

        // Parse user_id
        let user_id =
            Uuid::parse_str(&session.user_id).map_err(|_| Errors::SessionInvalidUserId)?;

        Ok(RequiredSession(SessionContext {
            user_id,
            session_id,
        }))
    }
}

/// Optional session extractor - returns None if session is not present or invalid
///
/// Use this in handlers that work with or without authentication:
/// ```
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
        // Extract AppState
        let app_state = AppState::from_ref(state);

        // Try to extract Cookies
        let Some(cookies) = parts.extensions.get::<Cookies>() else {
            return Ok(OptionalSession(None));
        };

        // Try to extract session_id from cookie
        let Some(cookie) = cookies.get(SESSION_COOKIE_NAME) else {
            return Ok(OptionalSession(None));
        };
        let session_id = cookie.value().to_string();

        // Try to get session from Redis
        let Ok(Some(session)) =
            SessionService::get_session(&app_state.redis_client, &session_id).await
        else {
            return Ok(OptionalSession(None));
        };

        // Try to parse user_id
        let Ok(user_id) = Uuid::parse_str(&session.user_id) else {
            return Ok(OptionalSession(None));
        };

        Ok(OptionalSession(Some(SessionContext {
            user_id,
            session_id,
        })))
    }
}