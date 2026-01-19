use crate::service::oauth::types::{GithubEmail, GithubUserInfo};
use axumkit_errors::errors::Errors;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl, basic::BasicClient,
};
use reqwest::Client as HttpClient;

const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_INFO_URL: &str = "https://api.github.com/user";
const GITHUB_USER_EMAILS_URL: &str = "https://api.github.com/user/emails";

/// GitHub OAuth 인증 URL을 생성합니다.
/// Returns: (auth_url, state, pkce_verifier)
pub fn generate_github_auth_url(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    state: String,
) -> Result<(String, String, String), Errors> {
    let auth_url =
        AuthUrl::new(GITHUB_AUTH_URL.to_string()).map_err(|_| Errors::OauthInvalidAuthUrl)?;
    let token_url =
        TokenUrl::new(GITHUB_TOKEN_URL.to_string()).map_err(|_| Errors::OauthInvalidTokenUrl)?;
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
        .add_scope(Scope::new("read:user".to_string()))
        .add_scope(Scope::new("user:email".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Ok((
        auth_url.to_string(),
        csrf_token.secret().clone(),
        pkce_verifier.secret().clone(),
    ))
}

/// Authorization code를 access token으로 교환합니다.
pub async fn exchange_github_code(
    http_client: &HttpClient,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
    pkce_verifier: &str,
) -> Result<String, Errors> {
    let auth_url =
        AuthUrl::new(GITHUB_AUTH_URL.to_string()).map_err(|_| Errors::OauthInvalidAuthUrl)?;
    let token_url =
        TokenUrl::new(GITHUB_TOKEN_URL.to_string()).map_err(|_| Errors::OauthInvalidTokenUrl)?;
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

/// Access token으로 GitHub 사용자 정보를 가져옵니다.
pub async fn fetch_github_user_info(
    http_client: &HttpClient,
    access_token: &str,
) -> Result<GithubUserInfo, Errors> {
    let response = http_client
        .get(GITHUB_USER_INFO_URL)
        .header("User-Agent", "axumkit-server")
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

    let user_info = serde_json::from_str::<GithubUserInfo>(&response_text)
        .map_err(|_| Errors::OauthUserInfoParseFailed(response_text))?;

    Ok(user_info)
}

/// Access token으로 GitHub 사용자의 이메일 목록을 가져옵니다.
pub async fn fetch_github_user_emails(
    http_client: &HttpClient,
    access_token: &str,
) -> Result<Vec<GithubEmail>, Errors> {
    let response = http_client
        .get(GITHUB_USER_EMAILS_URL)
        .header("User-Agent", "axumkit-server")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| Errors::OauthUserInfoFetchFailed)?;

    if !response.status().is_success() {
        return Err(Errors::OauthUserInfoFetchFailed);
    }

    let emails = response.json::<Vec<GithubEmail>>().await.map_err(|_| {
        Errors::OauthUserInfoParseFailed("Failed to parse GitHub emails".to_string())
    })?;

    Ok(emails)
}
