use crate::service::oauth::provider::config::OAuthProviderConfig;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl, basic::BasicClient,
};
use reqwest::Client as HttpClient;
use axumkit_errors::errors::Errors;

/// OAuth 인증 URL을 생성합니다.
/// Returns: (auth_url, state, pkce_verifier)
pub fn generate_auth_url<P: OAuthProviderConfig>(
    state: String,
) -> Result<(String, String, String), Errors> {
    let (client_id, client_secret, redirect_uri) = P::credentials();

    let auth_url =
        AuthUrl::new(P::AUTH_URL.to_string()).map_err(|_| Errors::OauthInvalidAuthUrl)?;
    let token_url =
        TokenUrl::new(P::TOKEN_URL.to_string()).map_err(|_| Errors::OauthInvalidTokenUrl)?;
    let redirect_url =
        RedirectUrl::new(redirect_uri.to_string()).map_err(|_| Errors::OauthInvalidRedirectUrl)?;

    let client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_client_secret(ClientSecret::new(client_secret.to_string()))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let mut auth_request = client.authorize_url(|| CsrfToken::new(state));
    for scope in P::SCOPES {
        auth_request = auth_request.add_scope(Scope::new(scope.to_string()));
    }
    let (auth_url, csrf_token) = auth_request.set_pkce_challenge(pkce_challenge).url();

    Ok((
        auth_url.to_string(),
        csrf_token.secret().clone(),
        pkce_verifier.secret().clone(),
    ))
}

/// Authorization code를 access token으로 교환합니다.
pub async fn exchange_code<P: OAuthProviderConfig>(
    http_client: &HttpClient,
    code: &str,
    pkce_verifier: &str,
) -> Result<String, Errors> {
    let (client_id, client_secret, redirect_uri) = P::credentials();

    let auth_url =
        AuthUrl::new(P::AUTH_URL.to_string()).map_err(|_| Errors::OauthInvalidAuthUrl)?;
    let token_url =
        TokenUrl::new(P::TOKEN_URL.to_string()).map_err(|_| Errors::OauthInvalidTokenUrl)?;
    let redirect_url =
        RedirectUrl::new(redirect_uri.to_string()).map_err(|_| Errors::OauthInvalidRedirectUrl)?;

    let client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_client_secret(ClientSecret::new(client_secret.to_string()))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url);

    let token_result = client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
        .request_async(http_client)
        .await
        .map_err(|_| Errors::OauthTokenExchangeFailed)?;

    Ok(token_result.access_token().secret().clone())
}
