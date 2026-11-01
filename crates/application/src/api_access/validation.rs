use crate::{ApplicationError, ApplicationResult, api_access::ApiScope};

pub const MAX_CLIENTS_PER_ORGANIZATION: u64 = 100;
pub const MAX_KEYS_PER_CLIENT: u64 = 100;
pub const MAX_QUOTA: i64 = 1_000_000_000_000;

pub fn name(value: &str) -> ApplicationResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 100 || value.chars().any(char::is_control) {
        return Err(ApplicationError::InvalidInput(
            "invalid API client or key name",
        ));
    }
    Ok(value.into())
}
pub fn scopes(values: &[ApiScope]) -> ApplicationResult<()> {
    if values.len() > 2
        || values
            .iter()
            .enumerate()
            .any(|(index, scope)| values[..index].contains(scope))
    {
        return Err(ApplicationError::InvalidInput(
            "unsupported API client scopes",
        ));
    }
    Ok(())
}
pub fn key_lifetime(days: i64) -> ApplicationResult<()> {
    if !(1..=365).contains(&days) {
        return Err(ApplicationError::InvalidInput(
            "API key lifetime must be between 1 and 365 days",
        ));
    }
    Ok(())
}
pub fn quota(minute: i64, month: i64) -> ApplicationResult<()> {
    if !(0..=MAX_QUOTA).contains(&minute) || !(0..=MAX_QUOTA).contains(&month) {
        return Err(ApplicationError::InvalidInput("invalid API client quota"));
    }
    Ok(())
}
