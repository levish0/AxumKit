use crate::connection::SeaweedFsClient;
use axumkit_entity::posts;
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashSet;

/// Batch size for listing objects from storage
const BATCH_SIZE: i32 = 1000;

/// Minimum age in hours before a blob can be considered orphaned
/// (to avoid race conditions with in-flight transactions)
const MIN_AGE_HOURS: i64 = 1;

/// Run orphaned blob cleanup
///
/// Finds storage keys in SeaweedFS that don't exist in posts table
/// and deletes them. Only deletes blobs older than MIN_AGE_HOURS to avoid
/// race conditions with in-flight transactions.
pub async fn run_cleanup_orphaned_blobs(db: &DatabaseConnection, storage: &SeaweedFsClient) {
    tracing::info!(
        batch_size = BATCH_SIZE,
        min_age_hours = MIN_AGE_HOURS,
        "Starting orphaned blob cleanup"
    );

    let cutoff_time = Utc::now() - Duration::hours(MIN_AGE_HOURS);
    let mut continuation_token: Option<String> = None;
    let mut total_checked = 0u64;
    let mut total_deleted = 0u64;
    let mut total_errors = 0u64;

    loop {
        // 1. List objects from storage
        let (objects, next_token) = match storage
            .list_objects(continuation_token.as_deref(), BATCH_SIZE)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(error = %e, "Failed to list objects from storage");
                break;
            }
        };

        if objects.is_empty() {
            break;
        }

        // 2. Filter to only old enough objects
        let old_objects: Vec<_> = objects
            .into_iter()
            .filter(|obj| obj.last_modified.map(|t| t < cutoff_time).unwrap_or(false))
            .collect();

        if !old_objects.is_empty() {
            let keys: Vec<String> = old_objects.iter().map(|o| o.key.clone()).collect();
            total_checked += keys.len() as u64;

            // 3. Check which keys exist in DB
            let existing_keys = match find_existing_storage_keys(db, &keys).await {
                Ok(keys) => keys,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to query existing storage keys");
                    break;
                }
            };

            // 4. Delete orphaned keys (keys that don't exist in DB)
            for obj in old_objects {
                if !existing_keys.contains(&obj.key) {
                    match storage.delete(&obj.key).await {
                        Ok(_) => {
                            total_deleted += 1;
                            tracing::debug!(key = %obj.key, "Deleted orphaned blob");
                        }
                        Err(e) => {
                            total_errors += 1;
                            tracing::warn!(key = %obj.key, error = %e, "Failed to delete orphaned blob");
                        }
                    }
                }
            }
        }

        // Continue to next page
        continuation_token = next_token;
        if continuation_token.is_none() {
            break;
        }
    }

    tracing::info!(
        total_checked = total_checked,
        total_deleted = total_deleted,
        total_errors = total_errors,
        "Orphaned blob cleanup completed"
    );
}

/// Find which storage keys exist in posts table
async fn find_existing_storage_keys(
    db: &DatabaseConnection,
    keys: &[String],
) -> Result<HashSet<String>, anyhow::Error> {
    if keys.is_empty() {
        return Ok(HashSet::new());
    }

    let results: Vec<String> = posts::Entity::find()
        .select_only()
        .column(posts::Column::StorageKey)
        .filter(posts::Column::StorageKey.is_in(keys.to_vec()))
        .into_tuple()
        .all(db)
        .await?;

    Ok(results.into_iter().collect())
}
