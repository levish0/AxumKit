// Common utilities for reindex jobs.

use crate::SearchClient;
use meilisearch_sdk::client::SwapIndexes;
use std::time::Duration;

/// Polling interval while waiting for the swap task
const SWAP_WAIT_INTERVAL: Duration = Duration::from_secs(1);
/// Max time to wait for the swap task (it queues behind all add_documents tasks)
const SWAP_WAIT_TIMEOUT: Duration = Duration::from_secs(300);

/// Temp index uid used to build a reindex before swapping it into place
pub fn reindex_temp_uid(index_uid: &str) -> String {
    format!("{index_uid}_reindex")
}

/// Atomically swap the freshly built temp index into place, then delete the
/// temp index, which holds the pre-reindex data after the swap.
///
/// MeiliSearch processes tasks in queue order, so the swap only runs after all
/// previously enqueued add_documents tasks for the temp index have completed.
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
            // The temp index now holds the old data; drop it.
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
        Ok(task) => {
            // The swap task itself failed, so no swap happened and the live
            // index is untouched. Failing the job lets NATS retry it safely.
            Err(anyhow::anyhow!("index swap task failed: {:?}", task))
        }
        Err(e) => {
            // Timeout or network error: the swap is already enqueued and will
            // still run. Do NOT fail the job - a retry would enqueue a second
            // swap and revert to the old data. The leftover temp index is
            // reset by the next reindex.
            tracing::warn!(
                index = index_uid,
                error = %e,
                "Could not confirm index swap completion; assuming it will complete"
            );
            Ok(())
        }
    }
}
