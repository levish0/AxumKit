//! Authentication service layer.
//!
//! Provides signup, login/logout, email/password flows, session lifecycle
//! management, and optional TOTP authentication flows.

pub mod audit;
pub mod change_email;
pub mod change_password;
pub mod confirm_email_change;
pub mod device;
pub mod forgot_password;
pub mod list_sessions;
pub mod login;
pub mod logout;
pub mod resend_verification_email;
pub mod reset_password;
pub mod revoke_session;
pub mod session;
pub mod session_types;
pub mod set_initial_password;
pub mod signup;
pub mod totp;
pub mod verify_email;

pub use login::LoginResult;
pub use session_types::{Session, SessionContext};
