use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::api_access::contracts::{ApiClientInput, ApiClientOutput},
};
use axum::{Extension, Json, extract::State, http::StatusCode};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(post, path="/api/v1/organizations/{organization_id}/api-clients", tag="API access", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path)), request_body=ApiClientInput, responses((status=201,body=ApiClientOutput)))]
pub async fn create_client(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<ApiClientInput>,
) -> Result<(StatusCode, Json<ApiClientOutput>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .app
                .api_access
                .create_client(
                    &actor,
                    organization_id,
                    input.name,
                    input.scopes.into_iter().map(Into::into).collect(),
                    trace,
                )
                .await?
                .into(),
        ),
    ))
}
