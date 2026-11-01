//! Constant-time secret comparison.

use subtle::ConstantTimeEq;

/// Compares fixed-size digests without early exit. Hashing still depends on input length.
pub fn constant_time_str_eq(left: &str, right: &str) -> bool {
    let left_digest = blake3::hash(left.as_bytes());
    let right_digest = blake3::hash(right.as_bytes());
    left_digest.as_bytes().ct_eq(right_digest.as_bytes()).into()
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
