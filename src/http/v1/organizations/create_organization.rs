use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, Session},
    v1::organizations::contracts::OrganizationOutput,
    validation,
};
use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct OrganizationInput {
    #[validate(custom(function = "validation::organization_name"))]
    /// Trimmed name must contain 1 to 100 Unicode characters.
    pub name: String,
}

#[utoipa::path(post, path="/api/v1/organizations", tag="Organizations", security(("cookieAuth"=[]),("bearerAuth"=[])), request_body=OrganizationInput, responses((status=201,body=OrganizationOutput)))]
pub async fn create_organization(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiJson(input): ApiJson<OrganizationInput>,
) -> Result<(StatusCode, Json<OrganizationOutput>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .app
                .organizations
                .create_organization(&actor, input.name)
                .await?
                .into(),
        ),
    ))
}
