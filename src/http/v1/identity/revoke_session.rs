use crate::http::{
    HttpState, cookies,
    error::ApiError,
    extract::{ApiPath, Session},
};
use axum::{
    Extension,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(delete, path="/api/v1/sessions/{session_id}", tag="Sessions", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("session_id"=Uuid,Path)), responses((status=204)))]
pub async fn revoke_session(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
    ApiPath(session_id): ApiPath<Uuid>,
) -> Result<Response, ApiError> {
    state
        .app
        .identity
        .revoke_session(&actor, session_id, trace)
        .await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    if session_id == actor.session_id() {
        cookies::clear_session(&mut response, &state.settings);
    }
    Ok(response)
}
