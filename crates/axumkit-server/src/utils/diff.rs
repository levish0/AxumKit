use similar::{ChangeTag, TextDiff};

pub struct CharsDiff {
    pub chars_added: i32,
    pub chars_removed: i32,
}

/// Calculate the number of characters added and removed between old and new text
pub fn calculate_chars_diff(old: &str, new: &str) -> CharsDiff {
    let diff = TextDiff::from_chars(old, new);

    let mut chars_added = 0i32;
    let mut chars_removed = 0i32;

    for change in diff.iter_all_changes() {
        let len = change.value().chars().count() as i32;
        match change.tag() {
            ChangeTag::Insert => chars_added += len,
            ChangeTag::Delete => chars_removed += len,
            ChangeTag::Equal => {}
        }
    }

    CharsDiff {
        chars_added,
        chars_removed,
    }
}
