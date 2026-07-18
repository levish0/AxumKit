use unicode_general_category::{GeneralCategory, get_general_category};
use validator::ValidationError;

/// Validates that a string is not blank (not empty after trimming whitespace)
pub fn validate_not_blank(s: &str) -> Result<(), ValidationError> {
    if s.trim().is_empty() {
        return Err(ValidationError::new("blank_string"));
    }
    Ok(())
}

/// Reserved handles that cannot be registered
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
    // Only allow ASCII alphanumeric + underscore.
    // This implicitly rejects: control chars, invisible Unicode, homoglyphs,
    // combining chars (Zalgo), RTL/LTR overrides, zero-width chars, etc.
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

/// Validates a display name (Twitter-style policy, no emoji).
///
/// Rules:
/// - No control characters (`Cc`), format/invisible characters (`Cf`),
///   surrogates (`Cs`), private-use characters (`Co`), line separators (`Zl`),
///   or paragraph separators (`Zp`)
/// - No emoji or miscellaneous symbols (`So` — covers 😀, 🎉, ©, ®, ™, etc.)
/// - No more than 2 consecutive non-spacing marks (blocks Zalgo text)
/// - No angle brackets (`<`, `>`) — the HTML tag-injection primitives. Names are
///   echoed into profile responses, the search index and email templates, so a
///   sink that forgets to escape must still never receive a `<script>` payload.
///   `&` and quotes stay allowed (legitimate in names — "Q&A", "O'Brien" — and
///   cannot open a tag on their own).
///
/// Unicode letters, spaces, and (other) punctuation are permitted.
pub fn validate_display_name(name: &str) -> Result<(), ValidationError> {
    let mut consecutive_combining: u32 = 0;

    for ch in name.chars() {
        // Defense-in-depth against HTML injection: `<`/`>` are Unicode category
        // Sm (math symbols), which the category filter below permits, so reject
        // them explicitly.
        if ch == '<' || ch == '>' {
            return Err(ValidationError::new("display_name_invalid_chars"));
        }

        let category = get_general_category(ch);

        match category {
            // Cc: control chars (U+0000–U+001F, U+007F–U+009F)
            // Cf: format chars — covers ALL of: soft hyphen, ZWS, ZWNJ, ZWJ,
            //     LTR/RTL marks, LTR/RTL overrides, BOM, word joiners, etc.
            // Cs: surrogates
            // Co: private use
            // Zl: line separator (U+2028)
            // Zp: paragraph separator (U+2029)
            // So: other symbols — covers emoji (😀 🎉 🚀 etc.) and misc symbols (© ® ™ ★ ♠ etc.)
            GeneralCategory::Control
            | GeneralCategory::Format
            | GeneralCategory::Surrogate
            | GeneralCategory::PrivateUse
            | GeneralCategory::LineSeparator
            | GeneralCategory::ParagraphSeparator
            | GeneralCategory::OtherSymbol => {
                return Err(ValidationError::new("display_name_invalid_chars"));
            }
            // Mn: non-spacing marks — Zalgo abuses these
            GeneralCategory::NonspacingMark => {
                consecutive_combining += 1;
                if consecutive_combining > 2 {
                    return Err(ValidationError::new("display_name_combining_chars"));
                }
            }
            _ => {
                consecutive_combining = 0;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_display_name;

    #[test]
    fn display_name_rejects_angle_brackets() {
        assert!(validate_display_name("<script>alert(1)</script>").is_err());
        assert!(validate_display_name("<img src=x onerror=x>").is_err());
        assert!(validate_display_name("a<b").is_err());
        assert!(validate_display_name("a>b").is_err());
    }

    #[test]
    fn display_name_allows_ampersand_and_quotes() {
        assert!(validate_display_name("Q&A enjoyer").is_ok());
        assert!(validate_display_name("O'Brien").is_ok());
        assert!(validate_display_name("The \"Real\" One").is_ok());
    }

    #[test]
    fn display_name_allows_plain_names() {
        assert!(validate_display_name("김철수").is_ok());
        assert!(validate_display_name("Jane Doe").is_ok());
    }
}
