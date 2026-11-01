use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, Session},
    v1::identity::session_responses::cleared_session_response,
    validation,
};
use axum::{Extension, extract::State, response::Response};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct SetInitialPasswordInput {
    #[validate(custom(function = "validation::new_password"))]
    /// At least 15 Unicode characters and at most 1024 UTF-8 bytes.
    #[schema(min_length = 15)]
    pub new_password: String,
}

/// Adds the first password to an account that signs in only through OIDC.
///
/// Requires authentication within the last five minutes. An account that
/// already has a password receives 409. Success revokes every session,
/// including the current one, and clears the session cookie.
#[utoipa::path(post, path="/api/v1/account/password/initial", tag="Account", security(("cookieAuth"=[]),("bearerAuth"=[])), request_body=SetInitialPasswordInput, responses((status=204)))]
pub async fn set_initial_password(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
    ApiJson(input): ApiJson<SetInitialPasswordInput>,
) -> Result<Response, ApiError> {
    state
        .app
        .identity
        .set_initial_password(&actor, input.new_password, trace)
        .await?;
    Ok(cleared_session_response(&state))
}
