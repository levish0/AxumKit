use axumkit_entity::common::OAuthProvider;

/// OAuth provider 설정을 정의하는 trait.
/// 각 provider(Google, GitHub 등)가 구현하여 generic 함수에서 사용합니다.
pub trait OAuthProviderConfig {
    const AUTH_URL: &'static str;
    const TOKEN_URL: &'static str;
    const SCOPES: &'static [&'static str];
    const PROVIDER: OAuthProvider;

    /// ServerConfig에서 (client_id, client_secret, redirect_uri)를 반환합니다.
    fn credentials() -> (&'static str, &'static str, &'static str);
}
