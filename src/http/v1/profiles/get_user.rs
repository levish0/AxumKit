use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
    v1::profiles::contracts::ProfileOutput,
};
use axum::{Json, extract::State};
use uuid::Uuid;

#[utoipa::path(get,path="/api/v1/organizations/{organization_id}/users/{account_id}",tag="Profiles",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),("account_id"=Uuid,Path)),responses((status=200,body=ProfileOutput)))]
pub async fn get_user(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, account_id)): ApiPath<(Uuid, Uuid)>,
) -> Result<Json<ProfileOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .profiles
            .get_user(&actor, organization_id, account_id)
            .await?
            .into(),
    ))
}
