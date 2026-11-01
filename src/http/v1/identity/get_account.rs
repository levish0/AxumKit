use crate::http::{HttpState, error::ApiError, extract::Session};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct AccountOutput {
    pub id: Uuid,
    pub email: String,
    pub verified: bool,
    pub mfa_enabled: bool,
    /// False for an account that signs in only through OIDC.
    pub has_password: bool,
}
impl From<application::identity::AccountSummary> for AccountOutput {
    fn from(account: application::identity::AccountSummary) -> Self {
        Self {
            id: account.id,
            email: account.email,
            verified: account.verified,
            mfa_enabled: account.mfa_enabled,
            has_password: account.has_password,
        }
    }
}

#[utoipa::path(get, path="/api/v1/account", tag="Account", security(("cookieAuth"=[]),("bearerAuth"=[])), responses((status=200,body=AccountOutput)))]
pub async fn get_account(
    State(state): State<HttpState>,
    Session(actor): Session,
) -> Result<Json<AccountOutput>, ApiError> {
    Ok(Json(state.app.identity.get_account(&actor).await?.into()))
}
