use crate::http::{HttpState, error::ApiError, extract::ApiPath};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct NativeOidcFlowOutput {
    /// Single-use secret for the completion request. Keep it in the native client.
    pub flow: String,
    /// Single-use nonce. Pass it unchanged to the provider SDK sign-in request.
    pub nonce: String,
}

#[utoipa::path(post, path="/api/v1/auth/oidc/{provider}/native", tag="OIDC", params(("provider"=String,Path)), responses((status=200,body=NativeOidcFlowOutput)))]
pub async fn begin_native_login(
    State(state): State<HttpState>,
    ApiPath(provider): ApiPath<String>,
) -> Result<Json<NativeOidcFlowOutput>, ApiError> {
    let authorization = state.app.oidc.begin_native_authorization(&provider).await?;
    Ok(Json(NativeOidcFlowOutput {
        flow: authorization.flow,
        nonce: authorization.nonce,
    }))
}
