use crate::{ApplicationError, ApplicationResult};

pub fn display_name(value: &str) -> ApplicationResult<()> {
    if value.trim().is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control)
    {
        return Err(ApplicationError::InvalidInput("invalid display name"));
    }
    Ok(())
}

pub fn bio(value: &str) -> ApplicationResult<()> {
    if value.chars().count() > 1000
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(ApplicationError::InvalidInput("invalid profile bio"));
    }
    Ok(())
}
