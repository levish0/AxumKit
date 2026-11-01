use crate::http::{
    HttpState, error::ApiError, extract::ApiKeyCredential,
    v1::api_access::quota_headers::quota_headers,
};
use axum::{Extension, Json, extract::State, http::HeaderMap};
use jobs::TraceContext;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ExternalOrganizationOutput {
    pub id: Uuid,
    pub name: String,
    pub version: i64,
}

#[utoipa::path(get, path="/api/v1/external/organization", tag="External API", security(("apiKeyAuth"=[])), responses((status=200,body=ExternalOrganizationOutput, headers(
    ("X-RateLimit-Limit"=i64, description="Per-client fixed UTC minute allowance"),
    ("X-RateLimit-Remaining"=i64, description="Remaining minute requests"),
    ("X-RateLimit-Reset"=i64, description="Minute reset Unix timestamp"),
    ("X-Quota-Limit"=i64, description="Per-client UTC calendar-month allowance"),
    ("X-Quota-Remaining"=i64, description="Remaining monthly requests"),
    ("X-Quota-Reset"=i64, description="Month reset Unix timestamp")
))))]
pub async fn external_organization(
    State(state): State<HttpState>,
    credential: ApiKeyCredential,
    Extension(trace): Extension<TraceContext>,
) -> Result<(HeaderMap, Json<ExternalOrganizationOutput>), ApiError> {
    let result = state
        .app
        .api_access
        .read_organization(credential.0.expose(), trace)
        .await?;
    let headers = quota_headers(&result.quota);
    Ok((
        headers,
        Json(ExternalOrganizationOutput {
            id: result.id,
            name: result.name,
            version: result.version,
        }),
    ))
}
