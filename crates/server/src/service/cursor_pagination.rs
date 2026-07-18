use std::future::Future;

/// Computes the cursors at both ends of the current page of items.
///
/// # Role
/// - Returns `(newer_cursor, older_cursor)` according to the cursor direction (`is_newer`).
/// - Returns `None` if the items are empty.
///
/// # Meaning of `is_newer` (caution)
/// This flag means "is the page sorted in **ascending order (oldest first)**"
/// (`true` means `last` is the most recent). Since `Newer` cursor queries are
/// ascending, it usually matches `cursor_direction == Newer`, but for **lists
/// whose default (no-cursor) order is ascending** (board comments, discussion
/// messages) the no-cursor page is also ascending, so `true` must be passed.
/// For lists with a descending default order, `cursor_direction == Newer` is
/// correct as-is.
///
/// # Related
/// - `service_list_reports`
/// - `service_get_action_logs`
/// - `service_list_moderation_logs`
/// - `service_get_discussions`
/// - `service_get_messages`
pub fn edge_cursors<T, K, F>(items: &[T], is_newer: bool, key: F) -> Option<(K, K)>
where
    K: Copy,
    F: Fn(&T) -> K,
{
    let (Some(first), Some(last)) = (items.first(), items.last()) else {
        return None;
    };

    let first_key = key(first);
    let last_key = key(last);

    if is_newer {
        Some((last_key, first_key))
    } else {
        Some((first_key, last_key))
    }
}

/// Restores `Newer`-direction query results to client display order.
///
/// # Role
/// Converts a newer page fetched in ascending order from the repository into
/// descending display order. Used for lists whose default order is
/// **descending (newest first)** (reports, logs, notifications, etc.).
///
/// # Related
/// - `edge_cursors`
/// - `reverse_if_older` (counterpart for ascending-default lists)
pub fn reverse_if_newer<T>(items: &mut [T], is_newer: bool) {
    if is_newer {
        items.reverse();
    }
}

/// Restores `Older`-direction query results to client display order.
///
/// # Role
/// For lists whose default order is **ascending (oldest first)** (discussion
/// messages, board comments), converts an older page fetched in descending
/// order by the repository into ascending display order. `Newer`/no-cursor
/// pages are already ascending, so they are left as-is — as a result the
/// order is always ascending regardless of pagination direction.
///
/// # Related
/// - `reverse_if_newer` (counterpart for descending-default lists)
pub fn reverse_if_older<T>(items: &mut [T], is_older: bool) {
    if is_older {
        items.reverse();
    }
}

/// Computes `has_newer`/`has_older` relative to the current page.
///
/// # Role
/// - Returns `(false, false)` for an empty page.
/// - Computes the cursors at both ends of the page, then runs the existence-check functions provided by the caller.
/// - Domain-specific filters/permissions/response mapping stay in each service; only cursor boundary computation is shared.
pub async fn cursor_flags<T, K, F, NewerFn, OlderFn, NewerFut, OlderFut, E>(
    items: &[T],
    is_newer: bool,
    key: F,
    exists_newer: NewerFn,
    exists_older: OlderFn,
) -> Result<(bool, bool), E>
where
    K: Copy,
    F: Fn(&T) -> K,
    NewerFn: FnOnce(K) -> NewerFut,
    OlderFn: FnOnce(K) -> OlderFut,
    NewerFut: Future<Output = Result<bool, E>>,
    OlderFut: Future<Output = Result<bool, E>>,
{
    let Some((newer_cursor, older_cursor)) = edge_cursors(items, is_newer, key) else {
        return Ok((false, false));
    };

    let has_newer = exists_newer(newer_cursor).await?;
    let has_older = exists_older(older_cursor).await?;

    Ok((has_newer, has_older))
}

#[cfg(test)]
mod tests {
    use super::{cursor_flags, edge_cursors, reverse_if_newer, reverse_if_older};

    #[derive(Clone, Copy)]
    struct Item {
        id: u32,
    }

    #[test]
    fn edge_cursors_returns_none_for_empty_items() {
        let items: Vec<Item> = vec![];
        assert_eq!(edge_cursors(&items, false, |item| item.id), None);
    }

    #[test]
    fn edge_cursors_uses_first_as_newer_and_last_as_older_when_direction_is_older() {
        let items = vec![Item { id: 30 }, Item { id: 20 }, Item { id: 10 }];
        assert_eq!(edge_cursors(&items, false, |item| item.id), Some((30, 10)));
    }

    #[test]
    fn edge_cursors_uses_last_as_newer_and_first_as_older_when_direction_is_newer() {
        let items = vec![Item { id: 10 }, Item { id: 20 }, Item { id: 30 }];
        assert_eq!(edge_cursors(&items, true, |item| item.id), Some((30, 10)));
    }

    #[test]
    fn reverse_if_newer_reverses_only_when_direction_is_newer() {
        let mut older_items = vec![3, 2, 1];
        reverse_if_newer(&mut older_items, false);
        assert_eq!(older_items, vec![3, 2, 1]);

        let mut newer_items = vec![1, 2, 3];
        reverse_if_newer(&mut newer_items, true);
        assert_eq!(newer_items, vec![3, 2, 1]);
    }

    #[test]
    fn reverse_if_older_reverses_only_when_direction_is_older() {
        // Older page fetched descending → normalized to ascending.
        let mut older_items = vec![3, 2, 1];
        reverse_if_older(&mut older_items, true);
        assert_eq!(older_items, vec![1, 2, 3]);

        // Newer / no-cursor page is already ascending → left untouched.
        let mut newer_items = vec![1, 2, 3];
        reverse_if_older(&mut newer_items, false);
        assert_eq!(newer_items, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn cursor_flags_returns_false_for_empty_items() {
        let items: Vec<Item> = vec![];
        let flags: Result<(bool, bool), ()> = cursor_flags(
            &items,
            false,
            |item| item.id,
            |_| async { Ok(true) },
            |_| async { Ok(true) },
        )
        .await;

        assert_eq!(flags.unwrap(), (false, false));
    }

    #[tokio::test]
    async fn cursor_flags_checks_newer_and_older_edges() {
        let items = vec![Item { id: 30 }, Item { id: 20 }, Item { id: 10 }];
        let flags: Result<(bool, bool), ()> = cursor_flags(
            &items,
            false,
            |item| item.id,
            |cursor| async move { Ok(cursor == 30) },
            |cursor| async move { Ok(cursor == 10) },
        )
        .await;

        assert_eq!(flags.unwrap(), (true, true));
    }
}
