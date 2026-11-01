use crate::{ApplicationError, ApplicationResult};

pub fn normalize_name(name: &str) -> ApplicationResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(ApplicationError::InvalidInput(
            "organization name must contain 1 to 100 characters",
        ));
    }
    Ok(name.into())
}
