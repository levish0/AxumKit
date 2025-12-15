use serde::Deserialize;

/// GitHub OAuth API에서 받는 유저 정보
#[derive(Debug, Deserialize)]
pub struct GithubUserInfo {
    /// GitHub user ID
    pub id: u64,
    /// GitHub username (login)
    pub login: String,
    /// Full name (optional)
    pub name: Option<String>,
    /// Email address (optional, may need separate API call)
    pub email: Option<String>,
    /// Avatar/profile image URL
    pub avatar_url: String,
}

/// GitHub user email 정보 (별도 API 호출용)
#[derive(Debug, Deserialize)]
pub struct GithubEmail {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}
