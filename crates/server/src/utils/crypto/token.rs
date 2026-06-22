use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngExt;

/// Generate a cryptographically secure token (32 bytes = 256 bits)
/// Returned as URL-safe Base64 encoding (43 characters)
pub fn generate_secure_token() -> String {
    let token_bytes: [u8; 32] = rand::rng().random();
    URL_SAFE_NO_PAD.encode(token_bytes)
}

/// Generate a cryptographically secure token of specified length
pub fn generate_secure_token_with_length(byte_length: usize) -> String {
    let token_bytes: Vec<u8> = (0..byte_length).map(|_| rand::rng().random()).collect();
    URL_SAFE_NO_PAD.encode(&token_bytes)
}

/// Hash a high-entropy token for storage/lookup so the raw token never lives at
/// rest (e.g. as a Redis key or index value). A store leak then yields only
/// hashes, which cannot be replayed against the endpoints.
///
/// The raw token is 256-bit CSPRNG output, so a fast cryptographic hash is the
/// right tool — there is nothing to brute-force, so a slow/salted password hash
/// (Argon2) would only add cost for no benefit. Returns a 64-char blake3 hex
/// digest. OWASP Forgot Password Cheat Sheet: store reset tokens hashed.
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

        // Two generated tokens should be different
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
