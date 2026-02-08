use crate::service::oauth::provider::config::OAuthProviderConfig;
use axumkit_config::ServerConfig;
use axumkit_entity::common::OAuthProvider;

pub struct GithubProvider;

impl OAuthProviderConfig for GithubProvider {
    const AUTH_URL: &'static str = "https://github.com/login/oauth/authorize";
    const TOKEN_URL: &'static str = "https://github.com/login/oauth/access_token";
    const SCOPES: &'static [&'static str] = &["read:user", "user:email"];
    const PROVIDER: OAuthProvider = OAuthProvider::Github;

    fn credentials() -> (&'static str, &'static str, &'static str) {
        let config = ServerConfig::get();
        (
            &config.github_client_id,
            &config.github_client_secret,
            &config.github_redirect_uri,
        )
    }
}
