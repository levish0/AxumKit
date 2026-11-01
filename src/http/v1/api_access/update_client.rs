use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::api_access::contracts::{ApiClientInput, ApiClientOutput},
};
use axum::{Extension, Json, extract::State};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(put, path="/api/v1/organizations/{organization_id}/api-clients/{client_id}", tag="API access", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("client_id"=Uuid,Path)), request_body=ApiClientInput, responses((status=200,body=ApiClientOutput)))]
pub async fn update_client(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, client_id)): ApiPath<(Uuid, Uuid)>,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<ApiClientInput>,
) -> Result<Json<ApiClientOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .api_access
            .update_client(
                &actor,
                organization_id,
                client_id,
                input.name,
                input.scopes.into_iter().map(Into::into).collect(),
                trace,
            )
            .await?
            .into(),
    ))
}
