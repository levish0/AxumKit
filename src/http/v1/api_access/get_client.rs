use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
    v1::api_access::contracts::ApiClientOutput,
};
use axum::{Json, extract::State};
use uuid::Uuid;

#[utoipa::path(get, path="/api/v1/organizations/{organization_id}/api-clients/{client_id}", tag="API access", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("client_id"=Uuid,Path)), responses((status=200,body=ApiClientOutput)))]
pub async fn get_client(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, client_id)): ApiPath<(Uuid, Uuid)>,
) -> Result<Json<ApiClientOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .api_access
            .get_client(&actor, organization_id, client_id)
            .await?
            .into(),
    ))
}
