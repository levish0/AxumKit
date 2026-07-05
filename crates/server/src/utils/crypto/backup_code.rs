//! TOTP backup-code hashing and verification.
//!
//! Backup codes are hashed with a keyed blake3 (the generic primitive lives in `auth-core`); this
//! app-layer adapter owns the domain-separation context and supplies the key from `TOTP_SECRET`.

use auth_core::constant_time::constant_time_str_eq;
use auth_core::keyed_hash;
use config::ServerConfig;

/// Domain-separation context for backup-code hashing. Owned by the app layer (kept out of the
/// generic `auth-core` primitive) and must stay stable — changing it invalidates stored hashes.
const BACKUP_CODE_CONTEXT: &str = "axumkit totp backup code v1";

/// Hash a backup code with a keyed blake3 hash, using `TOTP_SECRET` as the key.
pub fn hash_backup_code(code: &str) -> String {
    let key = ServerConfig::get().totp_secret.as_bytes();
    keyed_hash::hash_hex(key, BACKUP_CODE_CONTEXT, code.as_bytes())
}

/// Hash a list of backup codes.
pub fn hash_backup_codes(codes: &[String]) -> Vec<String> {
    codes.iter().map(|c| hash_backup_code(c)).collect()
}

/// Returns the index of the stored hash matching `code`, or `None`.
pub fn verify_backup_code(code: &str, stored_hashes: &[String]) -> Option<usize> {
    let input_hash = hash_backup_code(code);
    // Compare each stored digest in constant time. The digests are keyed-blake3 of the secret
    // code, so a plain `==` would be a timing oracle on the code's hash; `position` still
    // short-circuits, but only on which slot matched, not on the secret itself.
    stored_hashes
        .iter()
        .position(|h| constant_time_str_eq(h, &input_hash))
}
