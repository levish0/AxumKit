/// OAuth sign-in service result
pub enum SignInResult {
    /// Sign-in success (existing user or new user who provided a handle)
    Success(String), // session_id

    /// New user requested without a handle → pending signup state
    PendingSignup {
        pending_token: String,
        email: String,
    },
}
