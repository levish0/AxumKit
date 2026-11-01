use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, Session},
    v1::profiles::contracts::ProfileOutput,
    validation,
};
use application::profiles::SaveProfile;
use axum::{Extension, Json, extract::State};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileInput {
    #[validate(custom(function = "validation::display_name"))]
    #[schema(min_length = 1, max_length = 80)]
    pub display_name: String,
    #[validate(custom(function = "validation::profile_bio"))]
    #[schema(max_length = 1000)]
    pub bio: String,
    /// Zero creates a profile; subsequent changes require its current version.
    #[validate(range(min = 0))]
    pub expected_version: i64,
}

#[utoipa::path(put,path="/api/v1/account/profile",tag="Profiles",security(("cookieAuth"=[]),("bearerAuth"=[])),request_body=ProfileInput,responses((status=200,body=ProfileOutput)))]
pub async fn save_profile(
    State(state): State<HttpState>,
    Session(actor): Session,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<ProfileInput>,
) -> Result<Json<ProfileOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .profiles
            .save_profile(
                &actor,
                SaveProfile {
                    display_name: input.display_name,
                    bio: input.bio,
                    expected_version: input.expected_version,
                },
                trace,
            )
            .await?
            .into(),
    ))
}
