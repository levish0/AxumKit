use crate::http::{HttpState, error::ApiError, extract::Session};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ExternalIdentityOutput {
    pub id: Uuid,
    pub issuer: String,
}

#[utoipa::path(get, path="/api/v1/account/identities", tag="OIDC", security(("cookieAuth"=[]),("bearerAuth"=[])), responses((status=200,body=Vec<ExternalIdentityOutput>)))]
pub async fn list_identities(
    State(state): State<HttpState>,
    Session(actor): Session,
) -> Result<Json<Vec<ExternalIdentityOutput>>, ApiError> {
    Ok(Json(
        state
            .app
            .oidc
            .list_external_identities(&actor)
            .await?
            .into_iter()
            .map(|identity| ExternalIdentityOutput {
                id: identity.id,
                issuer: identity.issuer,
            })
            .collect(),
    ))
}
