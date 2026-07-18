use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Outcome of resolving an authenticated login against the trusted-device registry.
pub enum DeviceLoginOutcome {
    /// Trusted device (or app flow): a session was created; carries the raw session token.
    SessionCreated { session_token: String },
    /// New device: the session is withheld and a verification email was sent.
    VerificationRequired,
}

/// Redis payload backing a pending new-device verification (keyed by the emailed token's hash).
#[derive(Debug, Serialize, Deserialize)]
pub struct DevicePendingData {
    pub user_id: Uuid,
    pub remember_me: bool,
    /// The device-recognition token to trust once confirmed — the token the client presented
    /// (browser device cookie, or the app's `X-Device-Token` header), or a freshly minted one
    /// when the client presented none yet. The client stores it and re-presents it to skip the
    /// challenge next time.
    pub device_token: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}
