//! Encryption-at-rest for TOTP secrets.
//!
//! Unlike passwords (one-way hashed) or backup codes (keyed hash), the TOTP secret must be
//! recoverable to compute verification codes, so it is encrypted with AES-256-GCM. The generic
//! AEAD primitive lives in `auth-core`; this app-layer adapter owns the domain-separation context
//! and supplies the key from the configured `TOTP_ENCRYPTION_KEY` (which lives outside the
//! database — a DB-only leak no longer exposes 2FA seeds), mapping the primitive's errors onto
//! the application error type.

use auth_core::aead::{self, AeadError};
use config::ServerConfig;
use errors::errors::Errors;

/// Domain-separation context for the TOTP-secret encryption key. Owned by the app layer (kept out
/// of the generic `auth-core` primitive) and must stay stable — changing it breaks decryption.
const KEY_CONTEXT: &str = "axumkit totp secret encryption v1";

fn map_err(e: AeadError) -> Errors {
    let detail = match e {
        AeadError::Encrypt => "TOTP secret encryption failed",
        AeadError::Decrypt => "TOTP secret decryption failed",
        AeadError::Malformed => "TOTP secret malformed",
        AeadError::UnsupportedVersion => "TOTP secret unsupported version",
    };
    Errors::SysInternalError(detail.to_string())
}

/// Encrypts a TOTP secret (base32) for storage.
pub fn encrypt_totp_secret(secret_base32: &str) -> Result<String, Errors> {
    let key = ServerConfig::get().totp_encryption_key.as_bytes();
    aead::encrypt(key, KEY_CONTEXT, secret_base32.as_bytes()).map_err(map_err)
}

/// Decrypts a stored TOTP secret back to its base32 form.
pub fn decrypt_totp_secret(stored: &str) -> Result<String, Errors> {
    let key = ServerConfig::get().totp_encryption_key.as_bytes();
    let plaintext = aead::decrypt(key, KEY_CONTEXT, stored).map_err(map_err)?;
    String::from_utf8(plaintext)
        .map_err(|_| Errors::SysInternalError("TOTP secret invalid utf8".to_string()))
}
