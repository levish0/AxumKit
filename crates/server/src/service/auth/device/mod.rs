//! New-device login verification (OWASP ASVS 6.3.5).
//!
//! After credentials are fully verified, a login from an unrecognized device is held and challenged
//! by email on every channel; only a confirmed device is trusted and remembered — browsers remember
//! it via the device cookie, native apps via a stored `X-Device-Token`.

pub mod confirm;
pub mod resolve;
pub mod types;

pub use confirm::{DeviceVerifyResult, confirm_device_verification};
pub use resolve::resolve_device_login;
pub use types::{DeviceLoginOutcome, DevicePendingData};
