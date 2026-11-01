use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
};
use axum::{Extension, extract::State, http::StatusCode};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(delete, path="/api/v1/organizations/{organization_id}/api-clients/{client_id}", tag="API access", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("organization_id"=Uuid,Path),("client_id"=Uuid,Path)), responses((status=204)))]
pub async fn revoke_client(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, client_id)): ApiPath<(Uuid, Uuid)>,
    Extension(trace): Extension<TraceContext>,
) -> Result<StatusCode, ApiError> {
    state
        .app
        .api_access
        .revoke_client(&actor, organization_id, client_id, trace)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
