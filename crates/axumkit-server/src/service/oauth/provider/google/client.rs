use crate::service::oauth::types::GoogleUserInfo;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl, basic::BasicClient,
};
use reqwest::Client as HttpClient;
use axumkit_errors::errors::Errors;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USER_INFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

/// Google OAuth 인증 URL을 생성합니다.
/// Returns: (auth_url, state, pkce_verifier)
pub fn generate_google_auth_url(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    state: String,
) -> Result<(String, String, String), Errors> {
    let auth_url =
        AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|_| Errors::OauthInvalidAuthUrl)?;
    let token_url =
        TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|_| Errors::OauthInvalidTokenUrl)?;
    let redirect_url =
        RedirectUrl::new(redirect_uri.to_string()).map_err(|_| Errors::OauthInvalidRedirectUrl)?;

    let client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_client_secret(ClientSecret::new(client_secret.to_string()))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url);

    // PKCE challenge/verifier 쌍 생성
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(|| CsrfToken::new(state.clone()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Ok((
        auth_url.to_string(),
        csrf_token.secret().clone(),
        pkce_verifier.secret().clone(),
    ))
}

/// Authorization code를 access token으로 교환합니다.
pub async fn exchange_google_code(
    http_client: &HttpClient,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
    pkce_verifier: &str,
) -> Result<String, Errors> {
    let auth_url =
        AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|_| Errors::OauthInvalidAuthUrl)?;
    let token_url =
        TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|_| Errors::OauthInvalidTokenUrl)?;
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

/// Access token으로 Google 사용자 정보를 가져옵니다.
pub async fn fetch_google_user_info(
    http_client: &HttpClient,
    access_token: &str,
) -> Result<GoogleUserInfo, Errors> {
    let response = http_client
        .get(GOOGLE_USER_INFO_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| Errors::OauthUserInfoFetchFailed)?;

    if !response.status().is_success() {
        return Err(Errors::OauthUserInfoFetchFailed);
    }

    let response_text = response
        .text()
        .await
        .map_err(|_| Errors::OauthUserInfoFetchFailed)?;

    let user_info = serde_json::from_str::<GoogleUserInfo>(&response_text)
        .map_err(|_| Errors::OauthUserInfoParseFailed(response_text))?;

    Ok(user_info)
}
