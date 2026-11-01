use crate::http::{HttpState, error::ApiError, extract::ApiJson};
use axum::{Extension, extract::State, http::StatusCode};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct TokenInput {
    #[validate(length(min = 1, max = 1024))]
    #[schema(min_length = 1, max_length = 1024)]
    pub token: String,
}

#[utoipa::path(post, path="/api/v1/auth/verification/confirm", tag="Authentication", request_body=TokenInput, responses((status=204)))]
pub async fn verify_email(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<TokenInput>,
) -> Result<StatusCode, ApiError> {
    state.app.identity.verify_email(&input.token, trace).await?;
    Ok(StatusCode::NO_CONTENT)
}
