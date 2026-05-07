/// Redis key constants.

/// OAuth state key prefix (stores PKCE verifier)
/// Format: oauth:state:{uuid}
pub const OAUTH_STATE_KEY_PREFIX: &str = "oauth:state:";

/// OAuth pending signup key prefix
/// Format: oauth:pending:{uuid}
pub const OAUTH_PENDING_KEY_PREFIX: &str = "oauth:pending:";

/// OAuth pending signup lock key prefix
/// Format: oauth:pending:lock:{uuid}
pub const OAUTH_PENDING_LOCK_KEY_PREFIX: &str = "oauth:pending:lock:";

/// Build OAuth state key.
pub fn oauth_state_key(state: &str) -> String {
    format!("{}{}", OAUTH_STATE_KEY_PREFIX, state)
}

/// Build OAuth pending signup key.
pub fn oauth_pending_key(token: &str) -> String {
    format!("{}{}", OAUTH_PENDING_KEY_PREFIX, token)
}

/// Build OAuth pending signup lock key.
pub fn oauth_pending_lock_key(token: &str) -> String {
    format!("{}{}", OAUTH_PENDING_LOCK_KEY_PREFIX, token)
}
