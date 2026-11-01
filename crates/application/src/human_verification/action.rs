/// Names the operation that a Turnstile token protects.
///
/// Clients set the same value as the widget `action`. When action checks are
/// enabled, a token solved for one form cannot pass another operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HumanVerificationAction {
    Signup,
    EmailVerification,
    PasswordReset,
    Login,
}

impl HumanVerificationAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Signup => "signup",
            Self::EmailVerification => "email_verification",
            Self::PasswordReset => "password_reset",
            Self::Login => "login",
        }
    }
}
