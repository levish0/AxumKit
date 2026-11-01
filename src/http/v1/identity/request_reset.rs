use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ClientAddress},
    v1::identity::contracts::{AcceptedOutput, EmailInput},
};
use application::human_verification::HumanVerificationAction;
use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use jobs::TraceContext;

#[utoipa::path(post, path="/api/v1/auth/password/reset", tag="Authentication", request_body=EmailInput, responses((status=202,body=AcceptedOutput)))]
pub async fn request_reset(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    ClientAddress(client_address): ClientAddress,
    ApiJson(input): ApiJson<EmailInput>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .app
        .human_verification
        .verify(
            HumanVerificationAction::PasswordReset,
            input.human_verification_token.as_deref(),
            client_address,
        )
        .await?;
    state
        .app
        .identity
        .request_password_reset(&input.email, trace)
        .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(AcceptedOutput { accepted: true }),
    ))
}
