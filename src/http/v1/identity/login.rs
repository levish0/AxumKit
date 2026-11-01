use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ClientAddress},
    v1::identity::{
        contracts::{LoginOutput, SessionTransport},
        session_responses::{login_response, validate_transport},
    },
    validation,
};
use application::human_verification::HumanVerificationAction;
use axum::{Extension, extract::State, http::HeaderMap, response::Response};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct LoginInput {
    #[validate(custom(function = "validation::email"))]
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub transport: SessionTransport,
    /// Cloudflare Turnstile token from a widget with the action `login`.
    /// Required when the server enables human verification; ignored otherwise.
    #[validate(length(max = 2048))]
    #[schema(max_length = 2048)]
    pub human_verification_token: Option<String>,
}

#[utoipa::path(post, path="/api/v1/auth/login", tag="Authentication", request_body=LoginInput, responses((status=200,body=LoginOutput)))]
pub async fn login(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    headers: HeaderMap,
    ClientAddress(client_address): ClientAddress,
    ApiJson(input): ApiJson<LoginInput>,
) -> Result<Response, ApiError> {
    validate_transport(&state, &headers, input.transport)?;
    state
        .app
        .human_verification
        .verify(
            HumanVerificationAction::Login,
            input.human_verification_token.as_deref(),
            client_address,
        )
        .await?;
    let outcome = state
        .app
        .identity
        .login_with_password(&input.email, input.password, trace)
        .await?;
    Ok(login_response(&state, outcome, input.transport))
}
