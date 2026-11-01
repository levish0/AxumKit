use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
};
use axum::{Extension, extract::State, http::StatusCode};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(delete, path="/api/v1/organizations/{organization_id}/api-clients/{client_id}/keys/{key_id}", tag="API access", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("client_id"=Uuid,Path),("key_id"=Uuid,Path)), responses((status=204)))]
pub async fn revoke_key(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, client_id, key_id)): ApiPath<(Uuid, Uuid, Uuid)>,
    Extension(trace): Extension<TraceContext>,
) -> Result<StatusCode, ApiError> {
    state
        .app
        .api_access
        .revoke_key(&actor, organization_id, client_id, key_id, trace)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
