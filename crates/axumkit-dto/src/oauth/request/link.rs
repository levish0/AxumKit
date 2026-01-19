use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Google OAuth 연결 요청
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct GoogleLinkRequest {
    /// Authorization code from Google OAuth callback
    #[validate(length(min = 1, message = "Authorization code is required"))]
    pub code: String,

    /// State parameter for CSRF protection
    #[validate(length(min = 1, message = "State is required"))]
    pub state: String,
}

/// GitHub OAuth 연결 요청
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct GithubLinkRequest {
    /// Authorization code from GitHub OAuth callback
    #[validate(length(min = 1, message = "Authorization code is required"))]
    pub code: String,

    /// State parameter for CSRF protection
    #[validate(length(min = 1, message = "State is required"))]
    pub state: String,
}
