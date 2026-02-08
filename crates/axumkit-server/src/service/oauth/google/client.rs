use crate::service::oauth::types::GoogleUserInfo;
use reqwest::Client as HttpClient;
use axumkit_errors::errors::Errors;

const GOOGLE_USER_INFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

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
