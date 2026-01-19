use axumkit_constants::REALTIME_EVENTS_SUBJECT;
use axumkit_dto::action_logs::ActionLogResponse;
use futures::StreamExt;
use tokio::sync::broadcast;

/// Start NATS subscription and forward events to broadcast channel.
/// This runs as a background task, forwarding all events to SSE clients.
pub async fn start_eventstream_subscriber(
    nats_client: async_nats::Client,
    tx: broadcast::Sender<ActionLogResponse>,
) -> anyhow::Result<()> {
    let mut subscriber = nats_client.subscribe(REALTIME_EVENTS_SUBJECT).await?;

    tracing::info!(
        "EventStream subscriber started on subject: {}",
        REALTIME_EVENTS_SUBJECT
    );

    while let Some(msg) = subscriber.next().await {
        match serde_json::from_slice::<ActionLogResponse>(&msg.payload) {
            Ok(event) => {
                // Send to broadcast channel, ignore error if no receivers
                let _ = tx.send(event);
            }
            Err(e) => {
                tracing::warn!("Failed to deserialize event: {}", e);
            }
        }
    }

    tracing::warn!("EventStream subscriber ended unexpectedly");
    Ok(())
}
