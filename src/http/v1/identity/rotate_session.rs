use crate::http::{
    HttpState,
    error::ApiError,
    extract::Session,
    v1::identity::{
        contracts::{LoginOutput, SessionTransport},
        session_responses::session_response,
    },
};
use axum::{Extension, extract::State, http::HeaderMap, response::Response};
use jobs::TraceContext;

#[utoipa::path(post, path="/api/v1/sessions/current/rotate", tag="Sessions", security(("cookieAuth"=[]),("bearerAuth"=[])), responses((status=200,body=LoginOutput)))]
pub async fn rotate_session(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let transport = if headers.contains_key(axum::http::header::AUTHORIZATION) {
        SessionTransport::Bearer
    } else {
        SessionTransport::Cookie
    };
    let session = state.app.identity.rotate_session(&actor, trace).await?;
    Ok(session_response(&state, session, transport))
}
