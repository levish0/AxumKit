use axumkit_entity::common::OAuthProvider;

/// Trait defining OAuth provider configuration.
/// Implemented by each provider (Google, GitHub, etc.) for use in generic functions.
pub trait OAuthProviderConfig {
    const AUTH_URL: &'static str;
    const TOKEN_URL: &'static str;
    const SCOPES: &'static [&'static str];
    const PROVIDER: OAuthProvider;

    /// Returns (client_id, client_secret, redirect_uri) from ServerConfig.
    fn credentials() -> (&'static str, &'static str, &'static str);
}
