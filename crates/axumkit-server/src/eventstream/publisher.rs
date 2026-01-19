use axumkit_constants::REALTIME_EVENTS_SUBJECT;
use axumkit_dto::action_logs::ActionLogResponse;
use axumkit_entity::action_logs::Model as ActionLogModel;
use axumkit_errors::errors::Errors;

/// Publish an action log event to NATS for SSE distribution.
/// This is fire-and-forget - failures are logged but don't affect the main operation.
pub async fn publish_eventstream_event(
    nats_client: &async_nats::Client,
    action_log: &ActionLogModel,
) -> Result<(), Errors> {
    let event = ActionLogResponse::from(action_log.clone());
    let payload = serde_json::to_vec(&event).map_err(|_| Errors::EventStreamPublishFailed)?;

    nats_client
        .publish(REALTIME_EVENTS_SUBJECT, payload.into())
        .await
        .map_err(|_| Errors::EventStreamPublishFailed)?;

    Ok(())
}
