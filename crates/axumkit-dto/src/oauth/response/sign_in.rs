use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth::response::create_login_response;
use crate::oauth::internal::SignInResult;
use axumkit_errors::errors::Errors;

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

    /// Converts to HTTP response (remember_me=true for OAuth, 30-day session)
    pub fn into_response_result(self) -> Result<axum::response::Response, Errors> {
        match self {
            OAuthSignInResponse::Success { session_id } => {
                // OAuth sign-in always persists for 30 days (remember_me=true)
                create_login_response(session_id, true)
            }
            OAuthSignInResponse::PendingSignup(response) => Ok(response.into_response()),
        }
    }
}
