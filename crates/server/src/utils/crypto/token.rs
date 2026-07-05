//! Opaque token helpers.
//!
//! The generic primitives live in the `auth-core` crate; re-exported here so existing call sites
//! (`crate::utils::crypto::token::*`) keep working.

pub use auth_core::token::{generate_secure_token, generate_secure_token_with_length, hash_token};
