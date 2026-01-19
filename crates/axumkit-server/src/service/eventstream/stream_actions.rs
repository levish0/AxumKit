use crate::state::EventStreamSender;
use axum::response::sse::Event;
use axumkit_dto::action_logs::{ActionLogResponse, StreamActionsQuery};
use std::convert::Infallible;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

pub fn service_stream_actions(
    eventstream_tx: EventStreamSender,
    params: StreamActionsQuery,
) -> impl futures::Stream<Item = Result<Event, Infallible>> {
    let rx = eventstream_tx.subscribe();

    BroadcastStream::new(rx)
        .filter_map(move |result| {
            match result {
                Ok(event) => {
                    // Apply user_id filter (matches actor_id)
                    if let Some(user_id) = params.user_id {
                        if event.actor_id != Some(user_id) {
                            return None;
                        }
                    }
                    // Apply resource_id filter
                    if let Some(resource_id) = params.resource_id {
                        if event.resource_id != Some(resource_id) {
                            return None;
                        }
                    }
                    // Apply resource_type filter
                    if let Some(ref rt) = params.resource_type {
                        if &event.resource_type != rt {
                            return None;
                        }
                    }
                    // Apply actions filter
                    if let Some(ref actions) = params.actions {
                        if !actions.iter().any(|a| a.as_str() == event.action) {
                            return None;
                        }
                    }
                    Some(event)
                }
                Err(_) => None, // Lagged or closed, skip
            }
        })
        .map(|event: ActionLogResponse| {
            Ok(Event::default()
                .event("action_log")
                .id(event.id.to_string())
                .data(serde_json::to_string(&event).unwrap_or_default()))
        })
}
