use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use errors::errors::Errors;
use std::sync::LazyLock;

/// A real Argon2id hash to verify against when an account is missing or has no
/// password (OAuth-only). Verifying against it costs the same as a genuine
/// password check, so a failed login takes the same time no matter why it failed —
/// closing the account-enumeration timing side channel (OWASP Authentication Cheat
/// Sheet: uniform response time).
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password("auth-timing-dummy-password").expect("dummy hash is valid"));

/// Hard upper bound on password byte length, bounding the Argon2 input as a DoS
/// guard. The length *policy* (min/max characters) lives in the DTO validation
/// layer; this is a ceiling so an unvalidated path — e.g. the login candidate,
/// which is intentionally not length-validated — cannot feed a huge input into
/// Argon2. 1 KiB comfortably exceeds the 128-character policy.
pub const MAX_PASSWORD_BYTES: usize = 1024;

pub fn hash_password(password: &str) -> Result<String, Errors> {
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(Errors::BadRequestError(
            "Password exceeds the maximum allowed length.".to_string(),
        ));
    }

    // OWASP - Password Storage Cheat Sheet
    // Use Argon2id with a minimum configuration of 19 MiB of memory,
    // an iteration count of 2, and 1 degree of parallelism.
    let params = Params::new(
        19 * 1024, // 19 MiB memory (in KB)
        2,         // iterations
        1,         // parallelism
        None,      // output length default (32 bytes)
    )
    .map_err(|e| Errors::HashingError(e.to_string()))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::generate(&mut OsRng);

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Errors::HashingError(e.to_string()))?
        .to_string();

    Ok(password_hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<(), Errors> {
    // Reject over-long candidates as a plain mismatch (no Argon2 work, no oracle).
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(Errors::UserInvalidPassword);
    }

    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|e| Errors::HashingError(e.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| Errors::UserInvalidPassword)
}

/// Perform one Argon2 verification against a fixed dummy hash, discarding the result.
///
/// Call this on login paths that fail *before* a real password check — unknown email
/// or an account with no password (OAuth-only) — so every login attempt does exactly
/// one Argon2 verification and all failures take the same time. Without it, those paths
/// return early and are measurably faster than a wrong-password attempt, letting an
/// attacker enumerate which emails are real password accounts.
pub fn verify_dummy_password(password: &str) {
    let _ = verify_password(password, &DUMMY_HASH);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_hash_is_a_valid_argon2id_hash() {
        // The static dummy must parse and verify like any real hash, otherwise the
        // timing-equalizing path would do less work than a genuine verification.
        assert!(DUMMY_HASH.starts_with("$argon2id$"));
        assert!(verify_password("auth-timing-dummy-password", &DUMMY_HASH).is_ok());
    }

    #[test]
    fn verify_dummy_password_runs_for_any_input_without_panicking() {
        // Always performs a verification and never reveals the outcome to the caller.
        verify_dummy_password("anything");
        verify_dummy_password("");
    }

    #[test]
    fn hash_then_verify_roundtrips_and_rejects_wrong_password() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &hash).is_ok());
        assert!(matches!(
            verify_password("wrong", &hash),
            Err(Errors::UserInvalidPassword)
        ));
    }

    #[test]
    fn rejects_overlong_password() {
        let long = "a".repeat(MAX_PASSWORD_BYTES + 1);
        assert!(matches!(
            hash_password(&long),
            Err(Errors::BadRequestError(_))
        ));
        // Over-long verify candidate fails as a mismatch without hashing.
        assert!(matches!(
            verify_password(&long, &DUMMY_HASH),
            Err(Errors::UserInvalidPassword)
        ));
    }
}
