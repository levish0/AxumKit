use constants::NotificationAction;
use entity::notification_action_preferences::{
    ActiveModel as NotificationActionPreferenceActiveModel,
    Column as NotificationActionPreferenceColumn, Entity as NotificationActionPreferenceEntity,
    Model as NotificationActionPreferenceModel,
};
use errors::errors::Errors;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ConnectionTrait, EntityTrait, Set};
use uuid::Uuid;

/// Bulk-upserts a user's per-action notification preferences.
///
/// # Role
/// Inserts/updates the given list in one pass using `ON CONFLICT (user_id, action)`.
///
/// # Callers
/// - `service_update_action_preferences_bulk`
///
/// # Errors
/// - Returns a DB/repository error if the save fails.
pub async fn repository_upsert_action_preferences_bulk<C>(
    conn: &C,
    user_id: Uuid,
    preferences: Vec<(NotificationAction, bool)>,
) -> Result<Vec<NotificationActionPreferenceModel>, Errors>
where
    C: ConnectionTrait,
{
    // Deduplicate by action, keeping the last value. Postgres rejects an
    // ON CONFLICT batch that touches the same conflict target twice
    // ("ON CONFLICT DO UPDATE command cannot affect row a second time"), so a
    // request repeating an action would otherwise surface as a 500 for what is a
    // client-input problem.
    let preferences: Vec<(NotificationAction, bool)> = {
        let mut last_by_action: std::collections::HashMap<NotificationAction, bool> =
            std::collections::HashMap::with_capacity(preferences.len());
        for (action, enabled) in preferences {
            last_by_action.insert(action, enabled);
        }
        last_by_action.into_iter().collect()
    };

    if preferences.is_empty() {
        return Ok(vec![]);
    }

    // Prepare all active models
    let active_models: Vec<NotificationActionPreferenceActiveModel> = preferences
        .into_iter()
        .map(
            |(action, enabled)| NotificationActionPreferenceActiveModel {
                user_id: Set(user_id),
                action: Set(action.to_string()),
                enabled: Set(enabled),
                ..Default::default()
            },
        )
        .collect();

    // Use PostgreSQL ON CONFLICT for true bulk upsert (1 query instead of N)
    let results = NotificationActionPreferenceEntity::insert_many(active_models)
        .on_conflict(
            OnConflict::columns([
                NotificationActionPreferenceColumn::UserId,
                NotificationActionPreferenceColumn::Action,
            ])
            .update_column(NotificationActionPreferenceColumn::Enabled)
            .to_owned(),
        )
        .exec_with_returning(conn)
        .await?;

    Ok(results)
}
