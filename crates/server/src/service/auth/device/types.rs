use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Whether new-device verification applies to a login.
pub enum DeviceCheck {
    /// Browser flow: apply new-device verification. Carries the device-cookie token if the
    /// browser presented one.
    Browser(Option<String>),
    /// Native-app flow (no cookie jar): skip verification and mint the session directly.
    /// App clients are a separate device-trust model (no browser device cookie).
    Skip,
}

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
    /// The device-cookie token to trust once confirmed (existing cookie value, or a freshly
    /// minted one when the browser had no cookie yet).
    pub device_token: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}
