use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use chrono::Utc;
use std::convert::Infallible;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::service::auth::session::SessionService;
use crate::service::auth::session_types::SessionContext;
use crate::state::AppState;
use axumkit_errors::errors::Errors;

pub const SESSION_COOKIE_NAME: &str = "session_id";

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
        let session = SessionService::get_session(&app_state.redis_session, &session_id)
            .await?
            .ok_or(Errors::UserUnauthorized)?;

        // Check max lifetime (absolute expiration)
        if Utc::now() >= session.max_expires_at {
            return Err(Errors::SessionExpired);
        }

        // Conditionally refresh session (sliding expiration)
        // Errors are logged but don't fail the request
        if let Err(e) =
            SessionService::maybe_refresh_session(&app_state.redis_session, &session).await
        {
            tracing::warn!("Failed to refresh session: {:?}", e);
        }

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
            SessionService::get_session(&app_state.redis_session, &session_id).await
        else {
            return Ok(OptionalSession(None));
        };

        // Check max lifetime (absolute expiration)
        if Utc::now() >= session.max_expires_at {
            return Ok(OptionalSession(None));
        }

        // Conditionally refresh session (sliding expiration)
        // Errors are logged but don't fail the request
        if let Err(e) =
            SessionService::maybe_refresh_session(&app_state.redis_session, &session).await
        {
            tracing::warn!("Failed to refresh session: {:?}", e);
        }

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
