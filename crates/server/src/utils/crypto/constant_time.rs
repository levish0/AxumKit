//! Constant-time secret comparison.
//!
//! The generic primitive lives in the `auth-core` crate; re-exported here so existing call sites
//! (`crate::utils::crypto::constant_time::constant_time_str_eq`) keep working.

pub use auth_core::constant_time::constant_time_str_eq;
