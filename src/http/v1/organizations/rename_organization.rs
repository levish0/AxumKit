use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::organizations::contracts::OrganizationOutput,
    validation,
};
use axum::{Json, extract::State};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct RenameOrganizationInput {
    #[validate(custom(function = "validation::organization_name"))]
    /// Trimmed name must contain 1 to 100 Unicode characters.
    pub name: String,
    #[validate(range(min = 1))]
    #[schema(minimum = 1)]
    pub expected_version: i64,
}

#[utoipa::path(put, path="/api/v1/organizations/{organization_id}", tag="Organizations", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path)), request_body=RenameOrganizationInput, responses((status=200,body=OrganizationOutput)))]
pub async fn rename_organization(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<RenameOrganizationInput>,
) -> Result<Json<OrganizationOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .organizations
            .rename_organization(&actor, organization_id, input.expected_version, input.name)
            .await?
            .into(),
    ))
}
