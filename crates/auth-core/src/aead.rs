//! AES-256-GCM encryption-at-rest with a caller-supplied key and domain-separation context.
//!
//! Unlike passwords (one-way hashed) or backup codes (keyed hash), some secrets must be
//! recoverable (e.g. a TOTP seed used to compute verification codes), so they are encrypted rather
//! than hashed. The 32-byte AES key is derived from arbitrary-length `key_material` plus a
//! caller-owned `context` string, keeping this primitive free of any application-specific constant.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rand::RngExt;

const NONCE_LEN: usize = 12;
/// Ciphertext format version, prefixed to every blob so the key/scheme can be rotated later
/// without guess-and-decrypt (a stored blob self-describes which scheme produced it).
const FORMAT_VERSION: u8 = 1;

/// Errors from the AEAD primitive. Callers map these onto their own error type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AeadError {
    Encrypt,
    Decrypt,
    Malformed,
    UnsupportedVersion,
}

fn cipher(key_material: &[u8], context: &str) -> Aes256Gcm {
    // Derive a fixed 32-byte AES key from arbitrary-length key material + context.
    let key = blake3::derive_key(context, key_material);
    Aes256Gcm::new_from_slice(&key).expect("derive_key always yields 32 bytes")
}

/// Encrypts `plaintext` as `base64(version ‖ nonce ‖ ciphertext)`.
pub fn encrypt(key_material: &[u8], context: &str, plaintext: &[u8]) -> Result<String, AeadError> {
    let nonce_bytes: [u8; NONCE_LEN] = rand::rng().random();
    let nonce = Nonce::try_from(&nonce_bytes[..]).map_err(|_| AeadError::Encrypt)?;

    let ciphertext = cipher(key_material, context)
        .encrypt(&nonce, plaintext)
        .map_err(|_| AeadError::Encrypt)?;

    let mut combined = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len());
    combined.push(FORMAT_VERSION);
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(combined))
}

/// Decrypts a blob produced by [`encrypt`] with the same key material + context.
pub fn decrypt(key_material: &[u8], context: &str, stored: &str) -> Result<Vec<u8>, AeadError> {
    let combined = STANDARD.decode(stored).map_err(|_| AeadError::Malformed)?;
    // Layout: version(1) ‖ nonce(NONCE_LEN) ‖ ciphertext.
    if combined.len() <= 1 + NONCE_LEN {
        return Err(AeadError::Malformed);
    }

    let (version, rest) = combined.split_at(1);
    if version[0] != FORMAT_VERSION {
        return Err(AeadError::UnsupportedVersion);
    }

    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);
    let nonce = Nonce::try_from(nonce_bytes).map_err(|_| AeadError::Malformed)?;

    cipher(key_material, context)
        .decrypt(&nonce, ciphertext)
        .map_err(|_| AeadError::Decrypt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let blob = encrypt(b"key-material", "ctx v1", b"secret").unwrap();
        assert_eq!(
            decrypt(b"key-material", "ctx v1", &blob).unwrap(),
            b"secret"
        );
    }

    #[test]
    fn wrong_key_or_context_fails() {
        let blob = encrypt(b"key-material", "ctx v1", b"secret").unwrap();
        assert!(decrypt(b"other-key", "ctx v1", &blob).is_err());
        assert!(decrypt(b"key-material", "ctx v2", &blob).is_err());
    }

    #[test]
    fn rejects_malformed_and_unknown_version() {
        assert_eq!(
            decrypt(b"k", "c", "!!!notb64").err(),
            Some(AeadError::Malformed)
        );
        assert_eq!(decrypt(b"k", "c", "AAAA").err(), Some(AeadError::Malformed));
    }
}
