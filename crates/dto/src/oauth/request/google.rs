use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Google OAuth sign-in request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct GoogleLoginRequest {
    /// Authorization code from Google OAuth callback
    #[validate(length(min = 1, message = "Authorization code is required"))]
    pub code: String,

    /// State parameter for CSRF protection
    #[validate(length(min = 1, message = "State is required"))]
    pub state: String,
}

/// Google One Tap login request.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for signing in with Google One Tap.")]
pub struct GoogleOneTapLoginRequest {
    /// Google One Tap credential JWT
    #[validate(length(min = 1, message = "Credential is required"))]
    pub credential: String,
}
