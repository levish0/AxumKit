use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ClientAddress},
    v1::identity::contracts::AcceptedOutput,
    validation,
};
use application::human_verification::HumanVerificationAction;
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct RegisterInput {
    #[validate(custom(function = "validation::email"))]
    pub email: String,
    #[validate(custom(function = "validation::new_password"))]
    /// At least 15 Unicode characters and at most 1024 UTF-8 bytes.
    #[schema(min_length = 15)]
    pub password: String,
    /// Cloudflare Turnstile token from a widget with the action `signup`.
    /// Required when the server enables human verification; ignored otherwise.
    #[validate(length(max = 2048))]
    #[schema(max_length = 2048)]
    pub human_verification_token: Option<String>,
}

#[utoipa::path(post, path="/api/v1/auth/signup", tag="Authentication", request_body=RegisterInput, responses((status=202,body=AcceptedOutput)))]
pub async fn signup(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    ClientAddress(client_address): ClientAddress,
    ApiJson(input): ApiJson<RegisterInput>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .app
        .human_verification
        .verify(
            HumanVerificationAction::Signup,
            input.human_verification_token.as_deref(),
            client_address,
        )
        .await?;
    state
        .app
        .identity
        .register_account(&input.email, input.password, trace)
        .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(AcceptedOutput { accepted: true }),
    ))
}
