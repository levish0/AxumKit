use crate::extractors::RequiredSession;
use crate::service::user::update_my_profile::service_update_my_profile;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::user::UserResponse;
use axumkit_dto::user::request::UpdateMyProfileRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    patch,
    path = "/v0/user/me",
    request_body = UpdateMyProfileRequest,
    responses(
        (status = 200, description = "Profile updated successfully", body = UserResponse),
        (status = 400, description = "Bad Request - Validation error"),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 500, description = "Internal Server Error")
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User",
)]
pub async fn update_my_profile(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedJson(payload): ValidatedJson<UpdateMyProfileRequest>,
) -> Result<UserResponse, Errors> {
    service_update_my_profile(&state.write_db, &session_context, payload).await
}
