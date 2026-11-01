use crate::{ApplicationError, ApplicationResult};

pub fn validate_filename(filename: &str) -> ApplicationResult<()> {
    if filename.is_empty()
        || filename.len() > 255
        || filename
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\'))
        || matches!(filename, "." | "..")
    {
        return Err(ApplicationError::InvalidInput("invalid filename"));
    }
    Ok(())
}

pub fn validate_media_type(media_type: &str) -> ApplicationResult<()> {
    if media_type.len() > 120
        || !media_type.is_ascii()
        || media_type.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(ApplicationError::InvalidInput("invalid media type"));
    }
    Ok(())
}
