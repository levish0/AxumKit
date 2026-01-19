use axumkit_errors::errors::{Errors, ServiceResult};
use rand::Rng;
use totp_rs::{Algorithm, Secret, TOTP};

pub const ISSUER: &str = "Sevenwiki";
pub const BACKUP_CODE_COUNT: usize = 10;
pub const BACKUP_CODE_LENGTH: usize = 8;
const BACKUP_CODE_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// TOTP 코드 검증
pub fn verify_totp_code(secret_base32: &str, email: &str, code: &str) -> ServiceResult<bool> {
    let secret = Secret::Encoded(secret_base32.to_string())
        .to_bytes()
        .map_err(|_| Errors::TotpInvalidCode)?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some(ISSUER.to_string()),
        email.to_string(),
    )
    .map_err(|_| Errors::TotpInvalidCode)?;

    Ok(totp.check_current(code).unwrap_or(false))
}

/// 백업 코드 생성 (10개, 8자리 영숫자)
pub fn generate_backup_codes() -> Vec<String> {
    let mut rng = rand::rng();
    (0..BACKUP_CODE_COUNT)
        .map(|_| {
            (0..BACKUP_CODE_LENGTH)
                .map(|_| {
                    let idx = rng.random_range(0..BACKUP_CODE_CHARSET.len());
                    BACKUP_CODE_CHARSET[idx] as char
                })
                .collect()
        })
        .collect()
}
