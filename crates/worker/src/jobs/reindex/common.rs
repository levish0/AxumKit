// Common utilities for reindex jobs.

use crate::SearchClient;
use meilisearch_sdk::client::SwapIndexes;
use std::time::Duration;

/// Default batch size for reindex operations.
/// Kept modest because a whole batch is held in memory and sent to MeiliSearch
/// as a single payload.
pub const DEFAULT_BATCH_SIZE: u32 = 1_000;

const SWAP_WAIT_INTERVAL: Duration = Duration::from_secs(1);
const SWAP_WAIT_TIMEOUT: Duration = Duration::from_secs(300);

pub fn reindex_temp_uid(index_uid: &str) -> String {
    format!("{index_uid}_reindex")
}

/// Atomically swap the freshly built temp index into place, then delete the
/// temp index, which holds the pre-reindex data after the swap.
pub async fn promote_reindexed_index(
    client: &SearchClient,
    index_uid: &str,
) -> Result<(), anyhow::Error> {
    let temp_uid = reindex_temp_uid(index_uid);

    let swap_task = client
        .swap_indexes([&SwapIndexes {
            indexes: (index_uid.to_string(), temp_uid.clone()),
            rename: None,
        }])
        .await?;

    tracing::info!(index = index_uid, temp_index = %temp_uid, "Index swap enqueued");

    match swap_task
        .wait_for_completion(client, Some(SWAP_WAIT_INTERVAL), Some(SWAP_WAIT_TIMEOUT))
        .await
    {
        Ok(task) if task.is_success() => {
            if let Err(e) = client.delete_index(&temp_uid).await {
                tracing::warn!(
                    temp_index = %temp_uid,
                    error = %e,
                    "Failed to delete temp reindex index; next reindex will reset it"
                );
            }
            tracing::info!(index = index_uid, "Index swap completed");
            Ok(())
        }
        Ok(task) => Err(anyhow::anyhow!("index swap task failed: {:?}", task)),
        Err(e) => {
            // The swap is already enqueued. Retrying this job could enqueue a
            // second swap and restore the old data, so assume the queued task
            // will complete unless MeiliSearch reported a concrete failed task.
            tracing::warn!(
                index = index_uid,
                error = %e,
                "Could not confirm index swap completion; assuming it will complete"
            );
            Ok(())
        }
    }
}
