use crate::http::{
    HttpState, cookies,
    error::ApiError,
    extract::{ApiJson, Session},
    v1::identity::contracts::{MfaProofInput, RecoveryCodesOutput},
};
use axum::{
    Extension, Json,
    extract::State,
    response::{IntoResponse, Response},
};
use jobs::TraceContext;

#[utoipa::path(post, path="/api/v1/account/recovery-codes", tag="MFA", security(("cookieAuth"=[]),("bearerAuth"=[])), request_body=MfaProofInput, responses((status=200,body=RecoveryCodesOutput)))]
pub async fn regenerate_recovery_codes(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
    ApiJson(proof): ApiJson<MfaProofInput>,
) -> Result<Response, ApiError> {
    let recovery_codes = state
        .app
        .identity
        .regenerate_recovery_codes(&actor, proof.into(), trace)
        .await?;
    let mut response = Json(RecoveryCodesOutput { recovery_codes }).into_response();
    cookies::clear_session(&mut response, &state.settings);
    Ok(response)
}
