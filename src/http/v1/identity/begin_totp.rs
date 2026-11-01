use crate::http::{HttpState, error::ApiError, extract::Session};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct TotpEnrollmentOutput {
    pub challenge: String,
    pub authenticator_url: String,
}

#[utoipa::path(post, path="/api/v1/account/totp", tag="MFA", security(("cookieAuth"=[]),("bearerAuth"=[])), responses((status=200,body=TotpEnrollmentOutput)))]
pub async fn begin_totp(
    State(state): State<HttpState>,
    Session(actor): Session,
) -> Result<Json<TotpEnrollmentOutput>, ApiError> {
    let enrollment = state.app.identity.begin_totp_enrollment(&actor).await?;
    Ok(Json(TotpEnrollmentOutput {
        challenge: enrollment.challenge,
        authenticator_url: enrollment.authenticator_url,
    }))
}
