use crate::StorageClient;
use crate::jobs::WorkerContext;
use crate::nats::consumer::NatsConsumer;
use crate::nats::streams::{DELETE_CONTENT_CONSUMER, DELETE_CONTENT_STREAM};
use serde::{Deserialize, Serialize};

/// Storage content 삭제 Job
/// Document 삭제 또는 Revision 삭제 시 SeaweedFS에서 콘텐츠 삭제
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteContentJob {
    pub storage_keys: Vec<String>,
}

async fn handle_delete_content(
    job: DeleteContentJob,
    storage: &StorageClient,
) -> Result<(), anyhow::Error> {
    tracing::info!(
        "Processing delete content job: {} keys to delete",
        job.storage_keys.len()
    );

    let mut success_count = 0;
    let mut error_count = 0;

    for key in &job.storage_keys {
        match storage.delete(key).await {
            Ok(_) => {
                tracing::debug!(storage_key = %key, "Content deleted successfully");
                success_count += 1;
            }
            Err(e) => {
                tracing::warn!(storage_key = %key, error = %e, "Failed to delete content");
                error_count += 1;
            }
        }
    }

    tracing::info!(
        "Delete content job completed: {} succeeded, {} failed",
        success_count,
        error_count
    );

    // 일부 실패해도 job 자체는 성공으로 처리 (orphan cleanup이 나중에 잡아줌)
    Ok(())
}

/// Run the delete content consumer
pub async fn run_consumer(ctx: WorkerContext) -> anyhow::Result<()> {
    let storage_client = ctx.storage_client.clone();

    let consumer = NatsConsumer::new(
        ctx.jetstream.clone(),
        DELETE_CONTENT_STREAM,
        DELETE_CONTENT_CONSUMER,
        4, // concurrency
    );

    consumer
        .run::<DeleteContentJob, _, _>(move |job| {
            let storage = storage_client.clone();
            async move { handle_delete_content(job, &storage).await }
        })
        .await
}
