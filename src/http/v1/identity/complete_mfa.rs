use crate::http::{
    HttpState,
    error::ApiError,
    extract::ApiJson,
    v1::identity::{
        contracts::{LoginOutput, MfaProofInput, SessionTransport},
        session_responses::{session_response, validate_transport},
    },
};
use axum::{Extension, extract::State, http::HeaderMap, response::Response};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct MfaLoginInput {
    #[validate(length(min = 1, max = 1024))]
    #[schema(min_length = 1, max_length = 1024)]
    pub challenge: String,
    #[validate(nested)]
    pub proof: MfaProofInput,
    #[serde(default)]
    pub transport: SessionTransport,
}

#[utoipa::path(post, path="/api/v1/auth/mfa", tag="Authentication", request_body=MfaLoginInput, responses((status=200,body=LoginOutput)))]
pub async fn complete_mfa(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<MfaLoginInput>,
) -> Result<Response, ApiError> {
    validate_transport(&state, &headers, input.transport)?;
    let session = state
        .app
        .identity
        .complete_mfa_login(&input.challenge, input.proof.into(), trace)
        .await?;
    Ok(session_response(&state, session, input.transport))
}
