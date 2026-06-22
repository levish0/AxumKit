//! Email canonicalization.

/// Canonicalize an email address for storage and lookup: trim surrounding
/// whitespace and lowercase it, so case/whitespace variants resolve to one
/// account.
///
/// Applied at the repository boundary (every create/update/find), so all stored
/// and queried emails are canonical regardless of the entry point — form input,
/// OAuth provider profile, etc. The database additionally enforces case-insensitive
/// uniqueness via a `lower(email)` unique index as defense-in-depth.
///
/// Only case and surrounding whitespace are normalized; provider-specific rules
/// (e.g. Gmail dot/plus handling) are intentionally out of scope, since they vary
/// by provider and can merge addresses that are not actually equivalent.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_lowercases() {
        assert_eq!(normalize_email("  Foo@Example.COM "), "foo@example.com");
        assert_eq!(normalize_email("user@x.com"), "user@x.com");
    }

    #[test]
    fn case_and_whitespace_variants_canonicalize_equal() {
        assert_eq!(normalize_email("A@B.com"), normalize_email("a@b.com "));
    }
}
