//! Redis cache key prefixes and helpers
//! Centralized constants for cache key naming to ensure consistency across codebase

/// OAuth state TTL in seconds (5 minutes)
pub const OAUTH_STATE_TTL_SECONDS: u64 = 300;

/// OAuth state key prefix (stores PKCE verifier)
/// Format: "oauth:state:{uuid}"
pub const OAUTH_STATE_PREFIX: &str = "oauth:state:";

/// OAuth pending signup key prefix
/// Format: "oauth:pending:{uuid}"
pub const OAUTH_PENDING_PREFIX: &str = "oauth:pending:";

/// OAuth pending signup lock key prefix
/// Format: "oauth:pending:lock:{uuid}"
pub const OAUTH_PENDING_LOCK_PREFIX: &str = "oauth:pending:lock:";

/// Build OAuth state key.
pub fn oauth_state_key(state: &str) -> String {
    format!("{}{}", OAUTH_STATE_PREFIX, state)
}

/// Build OAuth pending-signup key.
pub fn oauth_pending_key(token: &str) -> String {
    format!("{}{}", OAUTH_PENDING_PREFIX, token)
}

/// Build OAuth pending-signup lock key.
pub fn oauth_pending_lock_key(token: &str) -> String {
    format!("{}{}", OAUTH_PENDING_LOCK_PREFIX, token)
}

/// Email verification token prefix.
/// Format: "email_verification:{token}"
pub const EMAIL_VERIFICATION_PREFIX: &str = "email_verification:";

/// Password reset token prefix.
/// Format: "password_reset:{token}"
pub const PASSWORD_RESET_PREFIX: &str = "password_reset:";

/// Email change token prefix.
/// Format: "email_change:{token}"
pub const EMAIL_CHANGE_PREFIX: &str = "email_change:";

/// Build email verification key.
pub fn email_verification_key(token: &str) -> String {
    format!("{}{}", EMAIL_VERIFICATION_PREFIX, token)
}

/// Build password reset key.
pub fn password_reset_key(token: &str) -> String {
    format!("{}{}", PASSWORD_RESET_PREFIX, token)
}

/// Build email change key.
pub fn email_change_key(token: &str) -> String {
    format!("{}{}", EMAIL_CHANGE_PREFIX, token)
}
