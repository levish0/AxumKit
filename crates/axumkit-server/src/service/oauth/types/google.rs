use serde::Deserialize;

/// User info received from the Google OAuth API
#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    /// Google user ID (v2 API uses 'id')
    pub id: String,
    /// Email address
    pub email: String,
    /// Email verification status
    pub verified_email: bool,
    /// Full name
    pub name: String,
    /// Given name (first name)
    pub given_name: String,
    /// Profile picture URL
    pub picture: String,
}
