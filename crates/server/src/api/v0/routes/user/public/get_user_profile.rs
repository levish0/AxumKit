use crate::service::user::public::get_user_profile_by_handle::service_get_user_profile_by_handle;
use crate::state::AppState;
use axum::extract::State;
use dto::user::{GetUserProfileRequest, PublicUserProfile};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/users/profile",
    summary = "Get a user profile",
    description = "Returns the public profile for the requested user handle.",
    params(GetUserProfileRequest),
    responses(
        (status = 200, description = "User profile retrieved successfully", body = PublicUserProfile),
        (status = 400, description = "Bad request - Invalid query parameters", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "User"
)]
pub async fn get_user_profile(
    State(state): State<AppState>,
    ValidatedQuery(payload): ValidatedQuery<GetUserProfileRequest>,
) -> Result<PublicUserProfile, Errors> {
    let profile = service_get_user_profile_by_handle(&state.db, &payload.handle).await?;
    Ok(profile)
}
