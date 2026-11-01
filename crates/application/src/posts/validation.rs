use crate::{ApplicationError, ApplicationResult};
use uuid::Uuid;

pub fn title(value: &str) -> ApplicationResult<()> {
    if value.trim().is_empty() || value.chars().count() > 200 || value.chars().any(char::is_control)
    {
        return Err(ApplicationError::InvalidInput("invalid post title"));
    }
    Ok(())
}
pub fn body(value: &str) -> ApplicationResult<()> {
    text(value, 8000, 12000)
}
pub fn comment(value: &str) -> ApplicationResult<()> {
    text(value, 2000, 8000)
}
fn text(value: &str, max_characters: usize, max_bytes: usize) -> ApplicationResult<()> {
    if value.trim().is_empty()
        || value.chars().count() > max_characters
        || value.len() > max_bytes
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(ApplicationError::InvalidInput("invalid content text"));
    }
    Ok(())
}
pub fn attachments(ids: &[Uuid]) -> ApplicationResult<()> {
    let mut unique = ids.to_vec();
    unique.sort();
    unique.dedup();
    if ids.len() > 10 || unique.len() != ids.len() {
        return Err(ApplicationError::InvalidInput("invalid attachment list"));
    }
    Ok(())
}
