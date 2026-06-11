use crate::state::EventStreamSender;
use axum::response::sse::Event;
use dto::action_logs::{ActionLogResponse, StreamActionsQuery};
use std::convert::Infallible;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

/// 필터 가능한 액션 로그 SSE 스트림을 생성한다.
///
/// # 역할
/// - 브로드캐스트 채널을 구독한다.
/// - 요청 파라미터 기준으로 이벤트를 필터링한다.
/// - `action_log` SSE 이벤트 포맷으로 직렬화해 반환한다.
///
/// # 연계
/// - `EventStreamSender`
/// - `StreamActionsQuery`
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
                    if let Some(user_id) = params.user_id
                        && event.actor_id != Some(user_id)
                    {
                        return None;
                    }
                    // Apply resource_id filter
                    if let Some(resource_id) = params.resource_id
                        && event.resource_id != Some(resource_id)
                    {
                        return None;
                    }
                    // Apply resource_type filter
                    if let Some(ref rt) = params.resource_type
                        && &event.resource_type != rt
                    {
                        return None;
                    }
                    // Apply actions filter
                    if let Some(ref actions) = params.actions
                        && !actions.iter().any(|a| a.as_str() == event.action)
                    {
                        return None;
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
