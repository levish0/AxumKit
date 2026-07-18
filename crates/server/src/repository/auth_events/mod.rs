//! Authentication audit-event repository (SEC-009).
//!
//! Durable, private-tier record of authentication decisions (login success/failure, logout,
//! credential/2FA changes). Recording is best-effort at call sites so a write failure never breaks
//! the authentication flow.

pub mod create;

pub use create::*;
