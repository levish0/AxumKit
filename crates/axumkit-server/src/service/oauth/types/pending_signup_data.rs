use axumkit_entity::common::OAuthProvider;
use serde::{Deserialize, Serialize};

/// Data temporarily stored in Redis when a new user requests OAuth sign-in without a handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSignupData {
    pub provider: OAuthProvider,
    pub provider_user_id: String,
    pub anonymous_user_id: String,
    pub email: String,
    pub profile_image: Option<String>,
}
