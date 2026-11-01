use crate::http::{
    HttpState, error::ApiError, extract::Session, v1::profiles::contracts::ProfileOutput,
};
use axum::{Json, extract::State};

#[utoipa::path(get,path="/api/v1/account/profile",tag="Profiles",security(("cookieAuth"=[]),("bearerAuth"=[])),responses((status=200,body=ProfileOutput)))]
pub async fn get_own_profile(
    State(state): State<HttpState>,
    Session(actor): Session,
) -> Result<Json<ProfileOutput>, ApiError> {
    Ok(Json(
        state.app.profiles.get_own_profile(&actor).await?.into(),
    ))
}
