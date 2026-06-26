use crate::service::oauth::types::{GithubEmail, GithubUserInfo};
use config::ServerConfig;
use errors::errors::Errors;
use reqwest::Client as HttpClient;
use serde::Deserialize;

const GITHUB_USER_INFO_URL: &str = "https://api.github.com/user";
const GITHUB_USER_EMAILS_URL: &str = "https://api.github.com/user/emails";

#[derive(Deserialize)]
struct GithubTokenCheckResponse {
    user: GithubUserInfo,
}

/// Verify that an app-submitted GitHub access token was issued for **our** OAuth app, via GitHub's
/// token introspection endpoint (`POST /applications/{client_id}/token`, authenticated with our
/// client id + secret).
///
/// A raw GitHub access token carries no verifiable audience, so calling `/user` with it would
/// happily accept a token minted for a *different* app (a confused-deputy / token-substitution
/// attack). This endpoint instead asks GitHub "was this token issued for the app whose
/// credentials I'm presenting?" — a foreign or revoked token returns 404 — which restores the
/// audience binding that Google's ID-token `aud` claim gives for free. Returns the authorizing
/// user from the introspection response.
pub async fn verify_github_token(
    http_client: &HttpClient,
    access_token: &str,
) -> Result<GithubUserInfo, Errors> {
    let config = ServerConfig::get();
    let url = format!(
        "https://api.github.com/applications/{}/token",
        config.github_client_id
    );

    let response = http_client
        .post(&url)
        .header("User-Agent", "AxumKit-server")
        .header("Accept", "application/vnd.github+json")
        .basic_auth(&config.github_client_id, Some(&config.github_client_secret))
        .json(&serde_json::json!({ "access_token": access_token }))
        .send()
        .await
        .map_err(|_| Errors::OauthUserInfoFetchFailed)?;

    // 404 = the token was not issued for our app (or is revoked); anything non-2xx is untrusted.
    if !response.status().is_success() {
        return Err(Errors::GithubInvalidToken);
    }

    let parsed = response
        .json::<GithubTokenCheckResponse>()
        .await
        .map_err(|_| {
            Errors::OauthUserInfoParseFailed("Failed to parse GitHub token check".to_string())
        })?;

    Ok(parsed.user)
}

/// Fetches GitHub user info using an access token.
pub async fn fetch_github_user_info(
    http_client: &HttpClient,
    access_token: &str,
) -> Result<GithubUserInfo, Errors> {
    let response = http_client
        .get(GITHUB_USER_INFO_URL)
        .header("User-Agent", "AxumKit-server")
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

/// Fetches the GitHub user's email list using an access token.
pub async fn fetch_github_user_emails(
    http_client: &HttpClient,
    access_token: &str,
) -> Result<Vec<GithubEmail>, Errors> {
    let response = http_client
        .get(GITHUB_USER_EMAILS_URL)
        .header("User-Agent", "AxumKit-server")
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
