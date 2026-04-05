use validator::ValidationError;

pub fn validate_not_blank(s: &str) -> Result<(), ValidationError> {
    if s.trim().is_empty() {
        return Err(ValidationError::new("blank_string"));
    }
    Ok(())
}

const RESERVED_HANDLES: &[&str] = &[
    "admin",
    "administrator",
    "support",
    "help",
    "system",
    "null",
    "undefined",
    "root",
    "moderator",
    "mod",
    "staff",
    "official",
    "bot",
    "api",
    "mail",
    "email",
    "info",
    "contact",
    "security",
    "abuse",
    "noreply",
    "no_reply",
    "anonymous",
    "guest",
    "user",
    "test",
];

/// Validates a user handle.
///
/// Rules:
/// - Only ASCII alphanumeric characters and underscores (`a-z`, `A-Z`, `0-9`, `_`)
/// - Cannot start or end with an underscore
/// - No consecutive underscores (`__`)
/// - Cannot be a reserved word (case-insensitive)
pub fn validate_handle(handle: &str) -> Result<(), ValidationError> {
    if !handle
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(ValidationError::new("handle_invalid_chars"));
    }

    if handle.starts_with('_') || handle.ends_with('_') {
        return Err(ValidationError::new("handle_leading_trailing_underscore"));
    }

    if handle.contains("__") {
        return Err(ValidationError::new("handle_consecutive_underscores"));
    }

    let lower = handle.to_ascii_lowercase();
    if RESERVED_HANDLES.contains(&lower.as_str()) {
        return Err(ValidationError::new("handle_reserved"));
    }

    Ok(())
}

/// Validates a display name.
///
/// Rules:
/// - Only Unicode letters, spaces, and common punctuation are allowed
/// - Rejects emoji, control characters, and invisible Unicode
pub fn validate_display_name(name: &str) -> Result<(), ValidationError> {
    for c in name.chars() {
        if c.is_alphanumeric() || c.is_ascii_punctuation() || c == ' ' {
            continue;
        }
        if c.is_control() {
            return Err(ValidationError::new("display_name_control_chars"));
        }
        // Allow general Unicode letters/marks (accented characters, CJK, etc.)
        if c.is_alphabetic() {
            continue;
        }
        return Err(ValidationError::new("display_name_invalid_chars"));
    }

    Ok(())
}
