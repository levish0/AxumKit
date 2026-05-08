use entity::common::OAuthProvider;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Data temporarily stored in Redis when a new user requests OAuth sign-in without a handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSignupData {
    pub provider: OAuthProvider,
    pub provider_user_id: String,
    pub anonymous_user_id: String,
    pub email: String,
    pub profile_image: Option<String>,
}

/// Redis state for an OAuth pending signup token.
///
/// Completed tokens are kept briefly so a client can retry after a response
/// timeout without creating a duplicate user or losing the session issuance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PendingSignupTokenState {
    Pending {
        data: PendingSignupData,
    },
    Completed {
        user_id: Uuid,
        provider: OAuthProvider,
        provider_user_id: String,
        anonymous_user_id: String,
    },
}
