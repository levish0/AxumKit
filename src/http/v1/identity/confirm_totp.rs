use crate::http::{
    HttpState, cookies,
    error::ApiError,
    extract::{ApiJson, Session},
    v1::identity::contracts::RecoveryCodesOutput,
};
use axum::{
    Extension, Json,
    extract::State,
    response::{IntoResponse, Response},
};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct TotpConfirmationInput {
    #[validate(length(min = 1, max = 1024))]
    #[schema(min_length = 1, max_length = 1024)]
    pub challenge: String,
    #[validate(length(min = 1, max = 1024))]
    #[schema(min_length = 1, max_length = 1024)]
    pub code: String,
}

#[utoipa::path(post, path="/api/v1/account/totp/confirm", tag="MFA", security(("cookieAuth"=[]),("bearerAuth"=[])), request_body=TotpConfirmationInput, responses((status=200,body=RecoveryCodesOutput)))]
pub async fn confirm_totp(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
    ApiJson(input): ApiJson<TotpConfirmationInput>,
) -> Result<Response, ApiError> {
    let recovery_codes = state
        .app
        .identity
        .confirm_totp_enrollment(&actor, &input.challenge, &input.code, trace)
        .await?;
    let mut response = Json(RecoveryCodesOutput { recovery_codes }).into_response();
    cookies::clear_session(&mut response, &state.settings);
    Ok(response)
}
