use async_nats::jetstream::Context as JetStream;
use serde::Serialize;

/// Publish a job to NATS JetStream (fire and forget)
pub async fn publish_job<T: Serialize>(
    jetstream: &JetStream,
    subject: &str,
    job: &T,
) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(job)?;

    jetstream
        .publish(subject.to_string(), payload.into())
        .await?
        .await?; // Wait for ack from server

    Ok(())
}

/// Publish a job without waiting for server ack (truly fire and forget)
pub async fn publish_job_no_ack<T: Serialize>(
    jetstream: &JetStream,
    subject: &str,
    job: &T,
) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(job)?;

    jetstream
        .publish(subject.to_string(), payload.into())
        .await?;

    Ok(())
}
