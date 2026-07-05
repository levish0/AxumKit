//! Constant-time secret comparison.

use subtle::ConstantTimeEq;

/// Constant-time equality for two secret strings, independent of their lengths.
///
/// Both inputs are hashed with blake3 first, so the comparison always runs over fixed-size
/// (32-byte) digests: it neither short-circuits on the first differing byte nor leaks the inputs'
/// lengths. Use this wherever a secret (shared secret, token, HMAC) is compared and a plain `==`
/// would be a timing oracle.
pub fn constant_time_str_eq(a: &str, b: &str) -> bool {
    let a = blake3::hash(a.as_bytes());
    let b = blake3::hash(b.as_bytes());
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::constant_time_str_eq;

    #[test]
    fn equal_strings_match() {
        assert!(constant_time_str_eq("s3cret", "s3cret"));
    }

    #[test]
    fn different_strings_do_not_match() {
        assert!(!constant_time_str_eq("s3cret", "wrong"));
        // Different lengths must not match and must not panic.
        assert!(!constant_time_str_eq("s3cret", "s3cretlonger"));
        assert!(!constant_time_str_eq("", "x"));
    }

    #[test]
    fn empty_strings_match() {
        assert!(constant_time_str_eq("", ""));
    }
}
