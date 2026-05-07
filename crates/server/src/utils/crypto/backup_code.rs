use config::ServerConfig;

/// Hash backup codes using Blake3 keyed hash
/// Uses TOTP_SECRET as the key to generate a cryptographically secure hash
pub fn hash_backup_code(code: &str) -> String {
    let config = ServerConfig::get();
    let key = blake3::derive_key("axumkit totp backup code v1", config.totp_secret.as_bytes());
    let mut hasher = blake3::Hasher::new_keyed(&key);
    hasher.update(code.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Hash and return a list of backup codes
pub fn hash_backup_codes(codes: &[String]) -> Vec<String> {
    codes.iter().map(|c| hash_backup_code(c)).collect()
}

/// Check if the input code matches any of the stored hashes
/// Returns the matching index if found, None otherwise
pub fn verify_backup_code(code: &str, stored_hashes: &[String]) -> Option<usize> {
    let input_hash = hash_backup_code(code);
    stored_hashes.iter().position(|h| h == &input_hash)
}
