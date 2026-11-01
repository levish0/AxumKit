//! Bounded Argon2id password hashing. Execute these CPU operations on a blocking pool.
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use rand::RngExt;

pub const MAX_PASSWORD_BYTES: usize = 1024;

#[derive(Debug, thiserror::Error)]
#[error("password hashing failed")]
pub struct PasswordError;

pub fn hash(password: &str) -> Result<String, PasswordError> {
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(PasswordError);
    }
    let params = Params::new(19 * 1024, 2, 1, None).map_err(|_| PasswordError)?;
    let salt_bytes: [u8; 16] = rand::rng().random();
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_with_salt(password.as_bytes(), &salt_bytes)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError)
}

pub fn verify(password: &str, encoded: &str) -> Result<bool, PasswordError> {
    if password.len() > MAX_PASSWORD_BYTES {
        return Ok(false);
    }
    let hash = PasswordHash::new(encoded).map_err(|_| PasswordError)?;
    match Argon2::default().verify_password(password.as_bytes(), &hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
        Err(_) => Err(PasswordError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn password_roundtrip_and_input_bound() {
        let hash = hash("correct horse battery staple").unwrap();
        assert!(verify("correct horse battery staple", &hash).unwrap());
        assert!(!verify("incorrect", &hash).unwrap());
        assert!(super::hash(&"x".repeat(MAX_PASSWORD_BYTES + 1)).is_err());
        assert!(verify("password", "broken stored hash").is_err());
    }
}
