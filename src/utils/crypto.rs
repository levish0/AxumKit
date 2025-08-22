use crate::service::error::errors::Errors;
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};

pub fn hash_password(password: &str) -> Result<String, Errors> {
    // OWASP - Password Storage Cheat Sheet
    // Use Argon2id with a minimum configuration of 19 MiB of memory,
    // an iteration count of 2, and 1 degree of parallelism.
    let params = Params::new(
        19 * 1024, // 19 MiB 메모리 (KB 단위)
        2,         // iterations
        1,         // parallelism
        None,      // output length 기본값 (32 bytes)
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
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|e| Errors::HashingError(e.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| Errors::UserInvalidPassword)
}