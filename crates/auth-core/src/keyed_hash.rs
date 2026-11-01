//! Keyed hashing with domain separation (blake3).

/// Returns a keyed digest under a caller-owned domain. Use Argon2id for passwords.
pub fn hash_hex(key_material: &[u8], context: &str, input: &[u8]) -> String {
    let key = blake3::derive_key(context, key_material);
    let mut hasher = blake3::Hasher::new_keyed(&key);
    hasher.update(input);
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::hash_hex;

    #[test]
    fn deterministic_and_hex() {
        let d = hash_hex(b"key", "ctx v1", b"code");
        assert_eq!(d, hash_hex(b"key", "ctx v1", b"code"));
        assert_eq!(d.len(), 64);
    }

    #[test]
    fn context_and_key_separate_the_domain() {
        assert_ne!(
            hash_hex(b"key", "ctx a", b"code"),
            hash_hex(b"key", "ctx b", b"code")
        );
        assert_ne!(
            hash_hex(b"key1", "ctx", b"code"),
            hash_hex(b"key2", "ctx", b"code")
        );
    }
}
