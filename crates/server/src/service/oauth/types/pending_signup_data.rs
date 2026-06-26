use entity::common::OAuthProvider;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Data temporarily stored in Redis when a new user requests OAuth sign-in without a handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSignupData {
    pub provider: OAuthProvider,
    pub provider_user_id: String,
    /// Browser-context binding for the completion step. `Some` for the redirect/One-Tap flow
    /// (the pending token must be completed from the same anonymous browser context, an extra
    /// CSRF defense for the redirect dance). `None` for the native-app `provider/token` flow,
    /// which has no cookie jar — there the single-use `pending_token` (delivered directly in the
    /// HTTPS response body, short TTL) is the binding, matching allauth's session-bound approach.
    pub anonymous_user_id: Option<String>,
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
        /// Same binding as the originating [`PendingSignupData::anonymous_user_id`] (see there).
        anonymous_user_id: Option<String>,
    },
}
