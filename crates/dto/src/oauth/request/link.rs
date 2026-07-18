use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Google OAuth link request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for linking a Google account to the current user.")]
pub struct GoogleLinkRequest {
    /// Authorization code from Google OAuth callback
    #[validate(length(min = 1, message = "Authorization code is required"))]
    pub code: String,

    /// State parameter for CSRF protection
    #[validate(length(min = 1, message = "State is required"))]
    pub state: String,
}

/// GitHub OAuth link request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for linking a GitHub account to the current user.")]
pub struct GithubLinkRequest {
    /// Authorization code from GitHub OAuth callback
    #[validate(length(min = 1, message = "Authorization code is required"))]
    pub code: String,

    /// State parameter for CSRF protection
    #[validate(length(min = 1, message = "State is required"))]
    pub state: String,
}
