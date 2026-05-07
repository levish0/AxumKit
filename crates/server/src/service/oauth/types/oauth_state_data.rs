use dto::oauth::request::OAuthAuthorizeFlow;
use entity::common::OAuthProvider;
use serde::{Deserialize, Serialize};

/// Data persisted in Redis for OAuth state validation.
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthStateData {
    /// PKCE code verifier required during token exchange.
    pub pkce_verifier: String,
    /// Expected OAuth flow for this state.
    pub flow: OAuthAuthorizeFlow,
    /// Expected OAuth provider for this state.
    pub provider: OAuthProvider,
    /// Browser binding value from anonymous cookie.
    pub anonymous_user_id: String,
}
