use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// GitHub OAuth sign-in request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for signing in with GitHub OAuth.")]
pub struct GithubLoginRequest {
    /// Authorization code from GitHub OAuth callback
    #[validate(length(min = 1, message = "Authorization code is required"))]
    pub code: String,

    /// State parameter for CSRF protection
    #[validate(length(min = 1, message = "State is required"))]
    pub state: String,
}

/// Native-app GitHub sign-in request (provider-token flow).
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for native-app GitHub sign-in with a provider access token.")]
pub struct GithubTokenRequest {
    /// GitHub OAuth access token obtained by the app via its own in-app authorization.
    // GitHub access tokens are short (`gho_…`); cap generously to bound abuse.
    #[validate(length(min = 1, max = 512, message = "Access token is required"))]
    pub access_token: String,
}
