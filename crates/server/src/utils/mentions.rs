//! Lightweight @handle mention extraction for user-authored content.
//!
//! Content is stored raw (no markup pipeline), so mentions are parsed
//! directly from the text: `@` followed by a handle-shaped token. Handles that
//! don't resolve to a user are simply ignored.

use crate::repository::user::repository_find_user_by_handle;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;
use std::collections::BTreeSet;
use uuid::Uuid;

/// Upper bound on resolved mentions per content body — keeps a pathological
/// post from turning into a notification broadcast.
const MAX_MENTIONS: usize = 10;

/// Extracts candidate handles from `@handle` tokens in the content.
///
/// A candidate must look like a valid handle (alphanumeric/underscore, no
/// leading/trailing underscore) and be delimited by a non-handle character.
fn extract_mention_handles(content: &str) -> Vec<String> {
    let mut handles = BTreeSet::new();
    let bytes = content.as_bytes();
    let is_handle_char = |c: u8| c.is_ascii_alphanumeric() || c == b'_';

    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' && (i == 0 || !is_handle_char(bytes[i - 1])) {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && is_handle_char(bytes[end]) {
                end += 1;
            }
            if end > start {
                let handle = &content[start..end];
                if !handle.starts_with('_') && !handle.ends_with('_') {
                    handles.insert(handle.to_string());
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }

    handles.into_iter().collect()
}

/// Resolves `@handle` mentions in `content` to user ids (deduplicated,
/// capped at [`MAX_MENTIONS`]). Unknown handles are ignored.
pub async fn resolve_mentions<C>(conn: &C, content: &str) -> Result<Vec<Uuid>, Errors>
where
    C: ConnectionTrait,
{
    let mut user_ids = Vec::new();
    for handle in extract_mention_handles(content) {
        if user_ids.len() >= MAX_MENTIONS {
            break;
        }
        if let Some(user) = repository_find_user_by_handle(conn, handle.clone()).await? {
            user_ids.push(user.id);
        }
    }
    Ok(user_ids)
}

#[cfg(test)]
mod tests {
    use super::extract_mention_handles;

    #[test]
    fn extracts_delimited_handles() {
        let handles = extract_mention_handles("hey @alice and @bob_1, not email@example.com");
        assert_eq!(handles, vec!["alice", "bob_1"]);
    }

    #[test]
    fn dedupes_and_ignores_bad_shapes() {
        let handles = extract_mention_handles("@alice @alice @_bad @bad_ @");
        assert_eq!(handles, vec!["alice"]);
    }
}
