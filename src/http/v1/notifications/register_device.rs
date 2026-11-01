use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, Session},
    v1::notifications::contracts::{PushDeviceOutput, PushPlatform},
    validation,
};
use axum::{Json, extract::State};
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PushDeviceInput {
    #[validate(custom(function = "validation::push_token"))]
    #[schema(min_length = 20, max_length = 4096)]
    pub token: String,
    pub platform: PushPlatform,
}

#[utoipa::path(post,path="/api/v1/account/push-devices",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),request_body=PushDeviceInput,responses((status=200,body=PushDeviceOutput)))]
pub async fn register_device(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiJson(input): ApiJson<PushDeviceInput>,
) -> Result<Json<PushDeviceOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .notifications
            .register_device(&actor, &input.token, input.platform.into())
            .await?
            .into(),
    ))
}
