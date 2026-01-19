use axumkit_config::ServerConfig;

/// 백업 코드를 Blake3 keyed hash로 해시
/// TOTP_SECRET을 키로 사용하여 암호학적으로 안전한 해시 생성
pub fn hash_backup_code(code: &str) -> String {
    let config = ServerConfig::get();
    let key = blake3::derive_key("axumkit totp backup code v1", config.totp_secret.as_bytes());
    let mut hasher = blake3::Hasher::new_keyed(&key);
    hasher.update(code.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// 백업 코드 목록을 해시하여 반환
pub fn hash_backup_codes(codes: &[String]) -> Vec<String> {
    codes.iter().map(|c| hash_backup_code(c)).collect()
}

/// 입력된 코드가 저장된 해시 목록 중 하나와 일치하는지 확인
/// 일치하면 해당 인덱스 반환, 없으면 None
pub fn verify_backup_code(code: &str, stored_hashes: &[String]) -> Option<usize> {
    let input_hash = hash_backup_code(code);
    stored_hashes.iter().position(|h| h == &input_hash)
}
