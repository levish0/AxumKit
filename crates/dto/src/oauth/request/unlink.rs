use entity::common::OAuthProvider;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// OAuth unlink request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[schema(description = "Request body for unlinking one OAuth provider from the current user.")]
pub struct UnlinkOAuthRequest {
    /// OAuth provider to unlink (Google or Github)
    pub provider: OAuthProvider,
}
