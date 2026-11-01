use crate::{ApplicationError, ApplicationResult};

pub fn normalize_email(email: &str) -> ApplicationResult<String> {
    let email = email.trim().to_ascii_lowercase();
    if email.len() > 254 || !mail::valid_recipient(&email) {
        return Err(ApplicationError::InvalidInput("invalid email address"));
    }
    Ok(email)
}

pub fn validate_new_password(password: &str) -> ApplicationResult<()> {
    if password.chars().count() < 15 || password.len() > auth_core::password::MAX_PASSWORD_BYTES {
        return Err(ApplicationError::InvalidInput(
            "password must contain at least 15 characters and at most 1024 bytes",
        ));
    }
    Ok(())
}
