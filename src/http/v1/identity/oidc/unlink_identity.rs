use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
};
use axum::{Extension, extract::State, http::StatusCode};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(delete, path="/api/v1/account/identities/{identity_id}", tag="OIDC", security(("cookieAuth"=[]),("bearerAuth"=[])), params(("identity_id"=Uuid,Path)), responses((status=204)))]
pub async fn unlink_identity(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    Session(actor): Session,
    ApiPath(identity_id): ApiPath<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .app
        .oidc
        .unlink_external_identity(&actor, identity_id, trace)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
