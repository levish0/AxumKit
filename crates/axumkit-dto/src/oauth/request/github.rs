use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// GitHub OAuth 로그인 요청
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct GithubLoginRequest {
    /// Authorization code from GitHub OAuth callback
    #[validate(length(min = 1, message = "Authorization code is required"))]
    pub code: String,

    /// State parameter for CSRF protection
    #[validate(length(min = 1, message = "State is required"))]
    pub state: String,
}
