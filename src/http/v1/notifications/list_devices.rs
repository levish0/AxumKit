use crate::http::{
    HttpState, error::ApiError, extract::Session, v1::notifications::contracts::PushDeviceOutput,
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct PushDeviceListOutput {
    pub items: Vec<PushDeviceOutput>,
}

#[utoipa::path(get,path="/api/v1/account/push-devices",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),responses((status=200,body=PushDeviceListOutput)))]
pub async fn list_devices(
    State(state): State<HttpState>,
    Session(actor): Session,
) -> Result<Json<PushDeviceListOutput>, ApiError> {
    Ok(Json(PushDeviceListOutput {
        items: state
            .app
            .notifications
            .list_devices(&actor)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    }))
}
