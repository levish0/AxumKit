/// Result of the OAuth sign-in service
pub enum SignInResult {
    /// Sign-in succeeded (existing user, or new user who provided a handle)
    Success(String), // session_id

    /// New user requested without a handle → pending signup state
    PendingSignup {
        pending_token: String,
        email: String,
    },
}
