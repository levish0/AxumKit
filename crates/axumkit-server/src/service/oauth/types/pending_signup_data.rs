use axumkit_entity::common::OAuthProvider;
use serde::{Deserialize, Serialize};

/// OAuth 로그인 시 신규 사용자가 handle 없이 요청한 경우 Redis에 임시 저장되는 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSignupData {
    pub provider: OAuthProvider,
    pub provider_user_id: String,
    pub email: String,
    pub display_name: String,
    pub profile_image: Option<String>,
}
