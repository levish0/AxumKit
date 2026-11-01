use crate::http::{
    HttpState, error::ApiError, extract::Session,
    v1::identity::session_responses::cleared_session_response,
};
use axum::{Extension, extract::State, response::Response};
use jobs::TraceContext;

#[utoipa::path(delete, path="/api/v1/sessions", tag="Sessions", security(("cookieAuth"=[]),("bearerAuth"=[])), responses((status=204)))]
pub async fn revoke_all_sessions(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
) -> Result<Response, ApiError> {
    state
        .app
        .identity
        .revoke_all_sessions(&actor, trace)
        .await?;
    Ok(cleared_session_response(&state))
}
