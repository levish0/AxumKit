use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
};
use axum::{extract::State, http::StatusCode};
use uuid::Uuid;

#[utoipa::path(delete,path="/api/v1/organizations/{organization_id}/members/{account_id}",tag="Organizations",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),("account_id"=Uuid,Path)),responses((status=204)))]
pub async fn remove_member(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, account_id)): ApiPath<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    state
        .app
        .organizations
        .remove_member(&actor, organization_id, account_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
