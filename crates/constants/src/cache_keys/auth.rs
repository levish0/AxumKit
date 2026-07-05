//! Authentication-related Redis cache keys (OAuth, email, password, TOTP, device).
//!
//! Token-derived keys take the **hashed** token id (see `utils::crypto::token::hash_token`), never
//! the raw token, so the raw token never lives at rest in Redis: a store snapshot yields only
//! non-replayable hashes.

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

/// TOTP used-code TTL in seconds. Covers a code's full acceptance window
/// (period 30s with skew ±1 ≈ 90s), after which the code is time-invalid anyway.
pub const TOTP_USED_CODE_TTL_SECONDS: u64 = 90;

/// Used-TOTP-code key prefix (single-use replay guard, per user + code).
/// Format: "totp:used:{user_id}:{code}"
pub const TOTP_USED_CODE_PREFIX: &str = "totp:used:";

/// Build used-TOTP-code key. A successfully verified TOTP code is claimed here so
/// the same code cannot be accepted twice within its validity window (RFC 6238 §5.2).
pub fn totp_used_code_key(user_id: &str, code: &str) -> String {
    format!("{}{}:{}", TOTP_USED_CODE_PREFIX, user_id, code)
}

/// Email verification token prefix.
/// Format: "email_verification:{blake3(token)}"
pub const EMAIL_VERIFICATION_PREFIX: &str = "email_verification:";

/// Password reset token prefix.
/// Format: "password_reset:{blake3(token)}"
pub const PASSWORD_RESET_PREFIX: &str = "password_reset:";

/// Email change token prefix.
/// Format: "email_change:{blake3(token)}"
pub const EMAIL_CHANGE_PREFIX: &str = "email_change:";

/// Account deletion confirmation token prefix.
/// Format: "account_deletion:{blake3(token)}"
pub const ACCOUNT_DELETION_PREFIX: &str = "account_deletion:";

/// New-device login verification token prefix.
/// Format: "device_verify:{blake3(token)}"
pub const DEVICE_VERIFY_PREFIX: &str = "device_verify:";

/// Build email verification key. Callers pass the hashed token id, never the raw token.
pub fn email_verification_key(token_id: &str) -> String {
    format!("{}{}", EMAIL_VERIFICATION_PREFIX, token_id)
}

/// Build password reset key. Callers pass the hashed token id, never the raw token.
pub fn password_reset_key(token_id: &str) -> String {
    format!("{}{}", PASSWORD_RESET_PREFIX, token_id)
}

/// Build email change key. Callers pass the hashed token id, never the raw token.
pub fn email_change_key(token_id: &str) -> String {
    format!("{}{}", EMAIL_CHANGE_PREFIX, token_id)
}

/// Build account-deletion confirmation key. Callers pass the hashed token id, never the raw token.
/// Holds the pending deletion until the emailed challenge is confirmed.
pub fn account_deletion_key(token_id: &str) -> String {
    format!("{}{}", ACCOUNT_DELETION_PREFIX, token_id)
}

/// Build new-device login verification key. Callers pass the hashed token id, never the raw token.
/// Holds the pending login until the emailed challenge is confirmed (OWASP ASVS 6.3.5).
pub fn device_verify_key(token_id: &str) -> String {
    format!("{}{}", DEVICE_VERIFY_PREFIX, token_id)
}

/// Pending email signup email index prefix
/// Format: "email_signup:email:{email}"
pub const EMAIL_SIGNUP_EMAIL_PREFIX: &str = "email_signup:email:";

/// Pending email signup handle index prefix
/// Format: "email_signup:handle:{handle}"
pub const EMAIL_SIGNUP_HANDLE_PREFIX: &str = "email_signup:handle:";

/// Build pending email-signup email index key.
pub fn email_signup_email_key(email: &str) -> String {
    format!("{}{}", EMAIL_SIGNUP_EMAIL_PREFIX, email)
}

/// Build pending email-signup handle index key.
pub fn email_signup_handle_key(handle: &str) -> String {
    format!("{}{}", EMAIL_SIGNUP_HANDLE_PREFIX, handle)
}
