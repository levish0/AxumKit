use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath},
    v1::identity::{
        contracts::{LoginOutput, SessionTransport},
        session_responses::login_response,
    },
};
use axum::{Extension, extract::State, response::Response};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct NativeOidcLoginInput {
    /// The `flow` value from the matching begin request.
    #[validate(length(min = 1, max = 128))]
    #[schema(min_length = 1, max_length = 128)]
    pub flow: String,
    /// The provider-signed ID token that the provider SDK returned for the flow nonce.
    #[validate(length(min = 1, max = 8192))]
    #[schema(min_length = 1, max_length = 8192)]
    pub id_token: String,
}

/// Native login always uses bearer transport, so the response includes the session token.
#[utoipa::path(post, path="/api/v1/auth/oidc/{provider}/native/complete", tag="OIDC", params(("provider"=String,Path)), request_body=NativeOidcLoginInput, responses((status=200,body=LoginOutput)))]
pub async fn complete_native_login(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    ApiPath(provider): ApiPath<String>,
    ApiJson(input): ApiJson<NativeOidcLoginInput>,
) -> Result<Response, ApiError> {
    let outcome = state
        .app
        .oidc
        .complete_native_authorization(&provider, &input.flow, &input.id_token, trace)
        .await?;
    Ok(login_response(&state, outcome, SessionTransport::Bearer))
}
