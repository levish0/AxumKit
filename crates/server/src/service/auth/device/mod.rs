//! New-device login verification (OWASP ASVS 6.3.5).
//!
//! After credentials are fully verified, a login from an unrecognized browser device is held and
//! challenged by email; only a confirmed device is trusted (and remembered via the device cookie).

pub mod confirm;
pub mod resolve;
pub mod types;

pub use confirm::{DeviceVerifyResult, confirm_device_verification};
pub use resolve::resolve_device_login;
pub use types::{DeviceCheck, DeviceLoginOutcome, DevicePendingData};
