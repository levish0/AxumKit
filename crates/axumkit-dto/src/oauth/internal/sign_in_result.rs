/// OAuth 로그인 서비스의 결과
pub enum SignInResult {
    /// 로그인 성공 (기존 사용자 또는 handle 제공한 신규 사용자)
    Success(String), // session_id

    /// 신규 사용자가 handle 없이 요청 → pending signup 상태
    PendingSignup {
        pending_token: String,
        email: String,
        display_name: String,
    },
}
