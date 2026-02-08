use crate::service::oauth::types::{GithubEmail, GithubUserInfo};
use axumkit_errors::errors::Errors;
use reqwest::Client as HttpClient;

const GITHUB_USER_INFO_URL: &str = "https://api.github.com/user";
const GITHUB_USER_EMAILS_URL: &str = "https://api.github.com/user/emails";

/// Access token으로 GitHub 사용자 정보를 가져옵니다.
pub async fn fetch_github_user_info(
    http_client: &HttpClient,
    access_token: &str,
) -> Result<GithubUserInfo, Errors> {
    let response = http_client
        .get(GITHUB_USER_INFO_URL)
        .header("User-Agent", "AxumKit")
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
        .header("User-Agent", "AxumKit")
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
