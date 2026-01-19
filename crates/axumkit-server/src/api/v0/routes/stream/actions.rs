use crate::service::eventstream::service_stream_actions;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axumkit_dto::action_logs::StreamActionsQuery;
use futures::Stream;
use std::convert::Infallible;
use std::time::Duration;

#[utoipa::path(
    get,
    path = "/v0/eventstream/actions",
    params(StreamActionsQuery),
    responses(
        (status = 200, description = "SSE stream of action logs", content_type = "text/event-stream"),
    ),
    tag = "EventStream"
)]
pub async fn stream_actions(
    State(state): State<AppState>,
    Query(params): Query<StreamActionsQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = service_stream_actions(state.eventstream_tx.clone(), params);
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
