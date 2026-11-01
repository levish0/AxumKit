use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
    v1::organizations::contracts::OrganizationOutput,
};
use axum::{Json, extract::State};
use uuid::Uuid;

#[utoipa::path(get, path="/api/v1/organizations/{organization_id}", tag="Organizations", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path)), responses((status=200,body=OrganizationOutput)))]
pub async fn get_organization(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
) -> Result<Json<OrganizationOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .organizations
            .get_organization(&actor, organization_id)
            .await?
            .into(),
    ))
}
