use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, Session},
    v1::identity::{contracts::MfaProofInput, session_responses::cleared_session_response},
};
use axum::{Extension, extract::State, response::Response};
use jobs::TraceContext;

#[utoipa::path(delete, path="/api/v1/account/totp", tag="MFA", security(("cookieAuth"=[]),("bearerAuth"=[])), request_body=MfaProofInput, responses((status=204)))]
pub async fn disable_totp(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
    ApiJson(proof): ApiJson<MfaProofInput>,
) -> Result<Response, ApiError> {
    state
        .app
        .identity
        .disable_totp(&actor, proof.into(), trace)
        .await?;
    Ok(cleared_session_response(&state))
}
