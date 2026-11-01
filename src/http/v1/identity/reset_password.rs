use crate::http::{
    HttpState, error::ApiError, extract::ApiJson,
    v1::identity::session_responses::cleared_session_response, validation,
};
use axum::{Extension, extract::State, response::Response};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct ResetPasswordInput {
    #[validate(length(min = 1, max = 1024))]
    #[schema(min_length = 1, max_length = 1024)]
    pub token: String,
    #[validate(custom(function = "validation::new_password"))]
    /// At least 15 Unicode characters and at most 1024 UTF-8 bytes.
    #[schema(min_length = 15)]
    pub new_password: String,
}

#[utoipa::path(post, path="/api/v1/auth/password/reset/confirm", tag="Authentication", request_body=ResetPasswordInput, responses((status=204)))]
pub async fn reset_password(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<ResetPasswordInput>,
) -> Result<Response, ApiError> {
    state
        .app
        .identity
        .reset_password(&input.token, input.new_password, trace)
        .await?;
    Ok(cleared_session_response(&state))
}
