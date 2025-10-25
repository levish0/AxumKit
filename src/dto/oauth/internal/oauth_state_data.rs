use serde::{Deserialize, Serialize};

/// OAuth State와 함께 Redis에 저장되는 PKCE verifier 데이터
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthStateData {
    /// PKCE code verifier (token exchange 시 필요)
    pub pkce_verifier: String,
}
