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
pub struct ChangePasswordInput {
    pub current_password: String,
    #[validate(custom(function = "validation::new_password"))]
    /// At least 15 Unicode characters and at most 1024 UTF-8 bytes.
    #[schema(min_length = 15)]
    pub new_password: String,
}

#[utoipa::path(put, path="/api/v1/account/password", tag="Account", security(("cookieAuth"=[]),("bearerAuth"=[])), request_body=ChangePasswordInput, responses((status=204)))]
pub async fn change_password(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
    ApiJson(input): ApiJson<ChangePasswordInput>,
) -> Result<Response, ApiError> {
    state
        .app
        .identity
        .change_password(&actor, input.current_password, input.new_password, trace)
        .await?;
    Ok(cleared_session_response(&state))
}
