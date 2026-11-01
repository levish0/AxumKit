//! High-entropy opaque tokens and fast hashing for storage/lookup.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngExt;

/// Generate a cryptographically secure token from 32 random bytes (256 bits).
/// Return it as 43 URL-safe Base64 characters.
pub fn generate_secure_token() -> String {
    let token_bytes: [u8; 32] = rand::rng().random();
    URL_SAFE_NO_PAD.encode(token_bytes)
}

/// Generate a cryptographically secure token of the given byte length.
pub fn generate_secure_token_with_length(byte_length: usize) -> String {
    let token_bytes: Vec<u8> = (0..byte_length).map(|_| rand::rng().random()).collect();
    URL_SAFE_NO_PAD.encode(&token_bytes)
}

/// Hash a random high-entropy token for lookup without storing its bearer value.
pub fn hash_token(token: &str) -> String {
    blake3::hash(token.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secure_token() {
        let token = generate_secure_token();
        // 32 bytes -> 43 chars in base64 (no padding)
        assert_eq!(token.len(), 43);

        let token2 = generate_secure_token();
        assert_ne!(token, token2);
    }

    #[test]
    fn test_generate_secure_token_with_length() {
        let token = generate_secure_token_with_length(16);
        // 16 bytes -> 22 chars in base64 (no padding)
        assert_eq!(token.len(), 22);
    }

    #[test]
    fn hash_token_is_deterministic_token_specific_and_hides_the_raw() {
        let token = generate_secure_token();
        assert_eq!(hash_token(&token), hash_token(&token));
        assert_ne!(hash_token(&token), hash_token("other"));
        // 32-byte blake3 digest -> 64 hex chars, and never the raw token.
        assert_eq!(hash_token(&token).len(), 64);
        assert_ne!(hash_token(&token), token);
    }
}
