use crate::{ApplicationError, ApplicationResult};

pub fn device_token(value: &str) -> ApplicationResult<()> {
    if !(20..=4096).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(ApplicationError::InvalidInput("invalid push device token"));
    }
    Ok(())
}
