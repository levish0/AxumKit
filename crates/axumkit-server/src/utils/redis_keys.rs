/// Redis key 상수

/// OAuth state 저장 키 (PKCE verifier 포함)
/// Format: oauth:state:{uuid}
pub const OAUTH_STATE_KEY_PREFIX: &str = "oauth:state:";

/// OAuth pending signup 저장 키
/// Format: oauth:pending:{uuid}
pub const OAUTH_PENDING_KEY_PREFIX: &str = "oauth:pending:";

/// OAuth state key 생성
pub fn oauth_state_key(state: &str) -> String {
    format!("{}{}", OAUTH_STATE_KEY_PREFIX, state)
}

/// OAuth pending signup key 생성
pub fn oauth_pending_key(token: &str) -> String {
    format!("{}{}", OAUTH_PENDING_KEY_PREFIX, token)
}
