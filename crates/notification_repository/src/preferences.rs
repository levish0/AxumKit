//! Per-action notification preference filtering (opt-out: an absent row means
//! enabled). Shared so the watcher fan-out's "who opted out of action X" logic
//! matches the direct-notification preference gate.

use constants::NotificationAction;
use entity::notification_action_preferences::{
    Column as NotificationActionPreferenceColumn, Entity as NotificationActionPreferenceEntity,
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashSet;
use uuid::Uuid;

/// From a candidate recipient list, drop the users who have explicitly disabled
/// `action` and return the rest. Preferences are opt-out — an absent row means
/// enabled — so only users with an `enabled = false` row are removed. The lookup
/// is chunked by `batch_size` to bound the `IN (...)` list size.
pub async fn filter_recipients_by_action_preference<C: ConnectionTrait>(
    conn: &C,
    candidate_user_ids: Vec<Uuid>,
    action: NotificationAction,
    batch_size: usize,
) -> Result<Vec<Uuid>, DbErr> {
    if candidate_user_ids.is_empty() {
        return Ok(candidate_user_ids);
    }

    let action = action.to_string();
    let mut disabled: HashSet<Uuid> = HashSet::new();
    for batch in candidate_user_ids.chunks(batch_size.max(1)) {
        let rows: Vec<(Uuid,)> = NotificationActionPreferenceEntity::find()
            .filter(NotificationActionPreferenceColumn::UserId.is_in(batch.to_vec()))
            .filter(NotificationActionPreferenceColumn::Action.eq(action.clone()))
            .filter(NotificationActionPreferenceColumn::Enabled.eq(false))
            .select_only()
            .column(NotificationActionPreferenceColumn::UserId)
            .into_tuple()
            .all(conn)
            .await?;

        disabled.extend(rows.into_iter().map(|(user_id,)| user_id));
    }

    Ok(candidate_user_ids
        .into_iter()
        .filter(|user_id| !disabled.contains(user_id))
        .collect())
}
