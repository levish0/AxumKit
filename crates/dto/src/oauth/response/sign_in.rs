use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth::response::{SessionTokenResponse, create_login_response};
use crate::oauth::internal::SignInResult;
use errors::errors::Errors;

/// Pending signup response returned when a new user signs in via OAuth without a handle
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "Response body returned when OAuth sign-in requires profile completion.")]
pub struct OAuthPendingSignupResponse {
    /// One-time token for completing pending signup
    pub pending_token: String,
    /// Email received from the OAuth provider
    pub email: String,
}

impl IntoResponse for OAuthPendingSignupResponse {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Converts OAuth sign-in result to HTTP response
pub enum OAuthSignInResponse {
    /// Sign-in success - 204 No Content + Set-Cookie
    Success { session_id: String },
    /// Pending signup - 200 OK + JSON body
    PendingSignup(OAuthPendingSignupResponse),
}

impl OAuthSignInResponse {
    /// Converts SignInResult to OAuthSignInResponse
    pub fn from_result(result: SignInResult) -> Self {
        match result {
            SignInResult::Success(session_id) => OAuthSignInResponse::Success { session_id },
            SignInResult::PendingSignup {
                pending_token,
                email,
            } => OAuthSignInResponse::PendingSignup(OAuthPendingSignupResponse {
                pending_token,
                email,
            }),
        }
    }

    /// Converts to HTTP response. OAuth has no remember-me input, so it issues a
    /// non-persistent browser session cookie (remember_me=false). The server's
    /// absolute session TTL still applies.
    pub fn into_response_result(self) -> Result<axum::response::Response, Errors> {
        match self {
            OAuthSignInResponse::Success { session_id } => {
                // No remember-me in the OAuth flow → non-persistent session cookie.
                create_login_response(session_id, false)
            }
            OAuthSignInResponse::PendingSignup(response) => Ok(response.into_response()),
        }
    }

    /// Native-app variant: an existing user receives the opaque session token in the response body
    /// (`Authorization: Bearer` use, no cookie); a new user receives the same pending-signup
    /// payload as the browser flow (already a JSON body) to finish via the app complete-signup.
    pub fn into_app_response_result(self) -> Result<axum::response::Response, Errors> {
        match self {
            OAuthSignInResponse::Success { session_id } => {
                Ok(SessionTokenResponse::new(session_id).into_response())
            }
            OAuthSignInResponse::PendingSignup(response) => Ok(response.into_response()),
        }
    }
}
