use crate::service::oauth::provider::config::OAuthProviderConfig;
use axumkit_config::ServerConfig;
use axumkit_entity::common::OAuthProvider;

pub struct GoogleProvider;

impl OAuthProviderConfig for GoogleProvider {
    const AUTH_URL: &'static str = "https://accounts.google.com/o/oauth2/v2/auth";
    const TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";
    const SCOPES: &'static [&'static str] = &["email", "profile"];
    const PROVIDER: OAuthProvider = OAuthProvider::Google;

    fn credentials() -> (&'static str, &'static str, &'static str) {
        let config = ServerConfig::get();
        (
            &config.google_client_id,
            &config.google_client_secret,
            &config.google_redirect_uri,
        )
    }
}
