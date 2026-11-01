use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::api_access::contracts::ApiKeyOutput,
    validation::api_access_name,
};
use axum::{Extension, Json, extract::State, http::StatusCode};
use jobs::TraceContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueApiKeyInput {
    #[validate(custom(function = "api_access_name"))]
    #[schema(min_length = 1, max_length = 100)]
    pub name: String,
    #[validate(range(min = 1, max = 365))]
    #[schema(minimum = 1, maximum = 365)]
    pub expires_in_days: i64,
}

#[derive(Serialize, ToSchema)]
pub struct IssuedApiKeyOutput {
    pub key: ApiKeyOutput,
    /// Returned once. Store securely; the server retains only a hash.
    pub credential: String,
}

#[utoipa::path(post, path="/api/v1/organizations/{organization_id}/api-clients/{client_id}/keys", tag="API access", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("client_id"=Uuid,Path)), request_body=IssueApiKeyInput, responses((status=201,body=IssuedApiKeyOutput)))]
pub async fn issue_key(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, client_id)): ApiPath<(Uuid, Uuid)>,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<IssueApiKeyInput>,
) -> Result<(StatusCode, Json<IssuedApiKeyOutput>), ApiError> {
    let issued = state
        .app
        .api_access
        .issue_key(
            &actor,
            organization_id,
            client_id,
            input.name,
            input.expires_in_days,
            trace,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(IssuedApiKeyOutput {
            key: issued.key.into(),
            credential: issued.credential.expose().into(),
        }),
    ))
}
