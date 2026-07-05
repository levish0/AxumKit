//! Generic authentication & cryptographic primitives.
//!
//! This crate is deliberately free of project-specific identifiers and application error types:
//! callers supply the key material and the domain-separation `context` strings, and map the small
//! local error types onto their own. Keeping these primitives generic lets them be reused and
//! audited independently of any one application (auth/security primitives stay generic — no
//! project names inside them).

pub mod aead;
pub mod constant_time;
pub mod keyed_hash;
pub mod token;
